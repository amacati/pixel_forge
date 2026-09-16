// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::ffi::c_void;
use std::ptr;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::{BOOL, HSTRING};
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONULL};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, FindWindowW, GetClientRect, GetDesktopWindow, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_CHILD, WS_EX_TOOLWINDOW,
};

use super::monitor::Monitor;

#[derive(thiserror::Error, Debug)]
pub enum WindowError {
    #[error("No active window found")]
    NoActiveWindow,
    #[error("Failed to find window with name '{0}'")]
    NotFound(String),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<WindowError> for PyErr {
    fn from(error: WindowError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

/// Decode a fixed size, NUL padded UTF-16 buffer as returned by the wide Win32 APIs.
pub(super) fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Window abstraction for the Windows operating system.
///
/// Windows can be used as capture target for the :class:`.Capture` class.
///
/// Args:
///     name: The name of the window.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[pyclass(from_py_object)]
pub struct Window {
    window_handle: isize,
}

#[pymethods]
impl Window {
    /// Create a :class:`.Window` instance from its name.
    ///
    /// Args:
    ///     name: The name of the window.
    ///
    /// Returns:
    ///    The window instance.
    ///
    /// Raises:
    ///    NotFound: The window with the given name was not found.
    #[new]
    pub fn new(name: &str) -> Result<Window, WindowError> {
        let wide_name = HSTRING::from(name);
        // SAFETY: `wide_name` is a NUL terminated wide string that outlives the call, and a null
        // class name matches any window class. The returned handle is borrowed, not owned by us.
        let window_handle = unsafe { FindWindowW(None, &wide_name) }
            .map_err(|_| WindowError::NotFound(String::from(name)))?;
        Ok(Self::from_handle(window_handle))
    }

    /// :``bool``: True if the window is still valid (i.e., open), else False.
    #[getter]
    pub fn valid(&self) -> bool {
        let handle = self.as_handle();
        // SAFETY: `handle` is a plain window handle. All calls only read window state and report
        // failure for stale or invalid handles. `rect` is a live, correctly sized out parameter for
        // the duration of the call.
        unsafe {
            let mut rect = RECT::default();
            if !IsWindowVisible(handle).as_bool() || GetClientRect(handle, &mut rect).is_err() {
                return false;
            }
            let styles = GetWindowLongPtrW(handle, GWL_STYLE);
            let ex_styles = GetWindowLongPtrW(handle, GWL_EXSTYLE);
            styles & WS_CHILD.0 as isize == 0 && ex_styles & WS_EX_TOOLWINDOW.0 as isize == 0
        }
    }

    /// :``str``: The name string of the window.
    #[getter]
    pub fn name(&self) -> String {
        let handle = self.as_handle();
        // SAFETY: `handle` is a plain window handle and both calls only read. The reported length
        // excludes the terminator, so the buffer holds the title plus its NUL. GetWindowTextW
        // never writes past the slice it is given and always NUL terminates.
        unsafe {
            let len = GetWindowTextLengthW(handle);
            if len <= 0 {
                return String::new();
            }
            let mut name = vec![0u16; len as usize + 1];
            GetWindowTextW(handle, &mut name);
            from_wide(&name)
        }
    }
}

impl Window {
    #[must_use]
    pub fn from_handle(window_handle: HWND) -> Window {
        Window {
            window_handle: window_handle.0 as isize,
        }
    }

    #[must_use]
    pub fn as_handle(&self) -> HWND {
        HWND(self.window_handle as *mut c_void)
    }

    /// The monitor with the largest area of intersection, or None if the window is off screen.
    #[must_use]
    pub fn monitor(&self) -> Option<Monitor> {
        // SAFETY: `as_handle()` is a plain window handle that is only read. MONITOR_DEFAULTTONULL
        // makes the call return a null handle instead of a fallback, which we check below.
        let monitor = unsafe { MonitorFromWindow(self.as_handle(), MONITOR_DEFAULTTONULL) };
        (!monitor.is_invalid()).then(|| Monitor::from_handle(monitor))
    }

    /// True if the window belongs to the calling process.
    fn own_process(&self) -> bool {
        let mut id = 0;
        // SAFETY: `as_handle()` is a plain window handle and `id` is a live out parameter for the
        // duration of the call. Both calls only read process information.
        unsafe {
            GetWindowThreadProcessId(self.as_handle(), Some(&mut id));
            id == GetCurrentProcessId()
        }
    }
}

// SAFETY: May only be passed to EnumChildWindows together with an LPARAM holding a pointer to a
// live, unaliased Vec<Window>, as done in enumerate_windows below.
unsafe extern "system" fn enum_windows_callback(window_handle: HWND, vec: LPARAM) -> BOOL {
    // SAFETY: `vec` is the pointer we handed to EnumChildWindows. It points to a Vec<Window> that
    // stays alive and is not borrowed elsewhere while the enumeration runs.
    let windows = unsafe { &mut *(vec.0 as *mut Vec<Window>) };

    let window = Window::from_handle(window_handle); // Not yet confirmed to be valid
    if window.valid() && !window.own_process() {
        windows.push(window);
    }

    TRUE
}

/// Enumerate all windows that are currently available.
///
/// Windows owned by the calling process are omitted.
///
/// Returns:
///     A list of all windows.
///
/// Raises:
///    WindowError: Enumerating the windows has failed.
#[pyfunction]
pub fn enumerate_windows() -> Result<Vec<Window>, WindowError> {
    let mut windows: Vec<Window> = Vec::new();
    // SAFETY: The callback matches the signature EnumChildWindows expects, and the LPARAM is a
    // pointer to `windows`, which outlives the enumeration and is not borrowed elsewhere. The
    // desktop window handle is valid for the lifetime of the process.
    unsafe {
        EnumChildWindows(
            Some(GetDesktopWindow()),
            Some(enum_windows_callback),
            LPARAM(ptr::addr_of_mut!(windows) as isize),
        )
        .ok()?;
    }
    Ok(windows)
}

/// Get the currently active window.
///
/// Returns:
///    The active window.
///
/// Raises:
///   NoActiveWindow: No active window was found.
#[pyfunction]
pub fn foreground_window() -> Result<Window, WindowError> {
    // SAFETY: The call takes no arguments and returns a borrowed handle, or null if no window is
    // active. We check for null before using it.
    let window_handle = unsafe { GetForegroundWindow() };
    if window_handle.0.is_null() {
        return Err(WindowError::NoActiveWindow);
    }
    Ok(Window::from_handle(window_handle))
}

// Window to GraphicsCaptureItem conversion
impl TryFrom<Window> for GraphicsCaptureItem {
    type Error = WindowError;

    fn try_from(value: Window) -> Result<Self, Self::Error> {
        let interop = windows::core::factory::<Self, IGraphicsCaptureItemInterop>()?;
        // SAFETY: `interop` is a live COM interface from the WinRT factory and the handle is a
        // plain window handle. CreateForWindow validates it and returns an error for an invalid
        // window. The returned capture item is owned by the caller.
        Ok(unsafe { interop.CreateForWindow(value.as_handle())? })
    }
}
