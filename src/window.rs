// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::ptr;
use std::string::FromUtf16Error;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::HSTRING;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONULL};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, FindWindowW, GetClientRect, GetDesktopWindow, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_CHILD, WS_EX_TOOLWINDOW,
};

use crate::monitor::Monitor;

#[derive(thiserror::Error, Debug)]
pub enum WindowError {
    #[error("No active window found")]
    NoActiveWindow,
    #[error("Failed to find window with name '{0}'")]
    NotFound(String),
    #[error("Failed to convert windows string '{0}' from UTF-16")]
    FailedToConvertWindowsString(#[from] FromUtf16Error),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<WindowError> for PyErr {
    fn from(error: WindowError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}
/// Window(title: str) -> Window
/// Window abstraction for the Windows operating system.
///
/// Windows can be used as capture target for the :class:`.Capture` class.
///
/// Args:
///     title: The title of the window.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[pyclass]
pub struct Window {
    window_handle: HWND,
}

#[pymethods]
impl Window {
    /// from_name(title: str) -> Window
    /// Create a :class:`.Window` instance from its name.
    ///
    /// Args:
    ///     title: The title of the window.
    ///
    /// Returns:
    ///    The window instance.
    ///
    /// Raises:
    ///    NotFound: The window with the given name was not found.
    #[new]
    pub fn new(title: &str) -> Result<Window, WindowError> {
        let hstring_title = HSTRING::from(title);
        let window_handle = unsafe { FindWindowW(None, &hstring_title) };

        if window_handle.0 == 0 {
            return Err(WindowError::NotFound(String::from(title)));
        }

        Ok(Window { window_handle })
    }

    /// :``bool``: True if the window is still valid (i.e., open), else False.
    #[getter]
    pub fn valid(&self) -> bool {
        if !unsafe { IsWindowVisible(self.window_handle).as_bool() } {
            return false;
        }

        let mut id = 0;
        unsafe { GetWindowThreadProcessId(self.window_handle, Some(&mut id)) };
        if id == unsafe { GetCurrentProcessId() } {
            return false;
        }

        let mut rect = RECT::default();
        let result = unsafe { GetClientRect(self.window_handle, &mut rect) };
        if result.is_ok() {
            let styles = unsafe { GetWindowLongPtrW(self.window_handle, GWL_STYLE) };
            let ex_styles = unsafe { GetWindowLongPtrW(self.window_handle, GWL_EXSTYLE) };

            if (ex_styles & isize::try_from(WS_EX_TOOLWINDOW.0).unwrap()) != 0 {
                return false;
            }
            if (styles & isize::try_from(WS_CHILD.0).unwrap()) != 0 {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    /// :``str``: The title string of the window.
    #[getter]
    pub fn title(&self) -> Result<String, WindowError> {
        let len = unsafe { GetWindowTextLengthW(self.window_handle) };

        let mut name = vec![0u16; usize::try_from(len).unwrap() + 1];
        if len >= 1 {
            let copied = unsafe { GetWindowTextW(self.window_handle, &mut name) };
            if copied == 0 {
                return Ok(String::new());
            }
        }

        let name = String::from_utf16(
            &name
                .as_slice()
                .iter()
                .take_while(|ch| **ch != 0x0000)
                .copied()
                .collect::<Vec<_>>(),
        )?;

        Ok(name)
    }
}

impl Window {
    /// Create a `Window` instance from a raw window handle (HWND).
    ///
    /// # Arguments
    ///
    /// * `window_handle` - The raw window handle (HWND).
    #[must_use]
    pub const fn from_handle(window_handle: HWND) -> Window {
        Window { window_handle }
    }

    /// Get the monitor that has the largest area of intersection with the window.
    ///
    /// # Returns
    ///
    /// `None` if the window doesn't intersect with any monitor.
    #[must_use]
    pub fn monitor(&self) -> Option<Monitor> {
        let monitor = unsafe { MonitorFromWindow(self.window_handle, MONITOR_DEFAULTTONULL) };

        if monitor.is_invalid() {
            None
        } else {
            Some(Monitor::from_handle(monitor))
        }
    }

    /// Return the window handle (HWND) of the window.
    #[must_use]
    pub const fn as_handle(&self) -> HWND {
        self.window_handle
    }
}

// Callback to enumerate all windows.
unsafe extern "system" fn enum_windows_callback(window_handle: HWND, vec: LPARAM) -> BOOL {
    let windows = &mut *(vec.0 as *mut Vec<Window>);

    let window = Window { window_handle }; // Not yet confirmed to be valid
    if window.valid() {
        windows.push(window);
    }

    TRUE
}

/// enumerate_windows() -> list[Window]
///
/// Enumerate all windows that are currently available.
///
/// Returns:
///     A list of all windows.
///
/// Raises:
///    WindowError: Enumerating the windows has failed.
#[pyfunction]
pub fn enumerate_windows() -> Result<Vec<Window>, WindowError> {
    let mut windows: Vec<Window> = Vec::new();

    unsafe {
        EnumChildWindows(
            GetDesktopWindow(),
            Some(enum_windows_callback),
            LPARAM(ptr::addr_of_mut!(windows) as isize),
        )
        .ok()?;
    };

    Ok(windows)
}

/// foreground_window() -> Window
///
/// Get the currently active window.
///
/// Returns:
///    The active window.
///
/// Raises:
///   NoActiveWindow: No active window was found.
#[pyfunction]
pub fn foreground_window() -> Result<Window, WindowError> {
    let window_handle = unsafe { GetForegroundWindow() };

    if window_handle.0 == 0 {
        return Err(WindowError::NoActiveWindow);
    }

    Ok(Window { window_handle })
}

// Window to GraphicsCaptureItem conversion
impl TryFrom<Window> for GraphicsCaptureItem {
    type Error = WindowError;

    fn try_from(value: Window) -> Result<Self, Self::Error> {
        let window_handle = value.as_handle();
        let interop = windows::core::factory::<Self, IGraphicsCaptureItemInterop>()?;

        Ok(unsafe { interop.CreateForWindow(window_handle)? })
    }
}
