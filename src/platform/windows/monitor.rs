// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::ffi::c_void;
use std::num::ParseIntError;
use std::ptr;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::BOOL;
use windows::core::{HSTRING, PCWSTR};
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW,
    MonitorFromPoint, DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFO,
    MONITORINFOEXW, MONITOR_DEFAULTTONULL,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use super::window::from_wide;

#[derive(thiserror::Error, Debug)]
pub enum MonitorError {
    #[error("Failed to find monitor")]
    NotFound,
    #[error("Monitor index is lower than one")]
    IndexError,
    #[error("Failed to get monitor info")]
    MonitorInfoError,
    #[error("Failed to get monitor settings")]
    MonitorSettingsError,
    #[error("Failed to get monitor name")]
    MonitorNameError,
    #[error("Failed to parse monitor index: {0}")]
    MonitorIndexError(#[from] ParseIntError),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<MonitorError> for PyErr {
    fn from(error: MonitorError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

/// Monitor class for the Windows operating system.
///
/// Monitor can be used as capture target for the :class:`.Capture` class. It also provides some
/// convenience methods to get information about the monitor.
///
/// Args:
///    id: The index of the monitor. If None, the primary monitor is used.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[pyclass(from_py_object)]
pub struct Monitor {
    monitor_handle: isize,
}

#[pymethods]
impl Monitor {
    /// Create a :class:`.Monitor` instance.
    ///
    /// Args:
    ///    id: The monitor ID. Monitor IDs start at 1. If None, the primary monitor is used.
    ///
    /// Raises:
    ///    IndexError: The monitor ID is lower than one.
    ///    NotFound: No monitor with the given ID exists.
    #[new]
    #[pyo3(signature = (id=None))]
    pub fn new(id: Option<usize>) -> Result<Self, MonitorError> {
        match id {
            Some(id) => Self::from_index(id),
            None => primary_monitor(),
        }
    }

    /// :``int``: The pixel width of the monitor.
    #[getter]
    pub fn width(&self) -> Result<u32, MonitorError> {
        Ok(self.display_settings()?.dmPelsWidth)
    }

    /// :``int``: The pixel height of the monitor.
    #[getter]
    pub fn height(&self) -> Result<u32, MonitorError> {
        Ok(self.display_settings()?.dmPelsHeight)
    }

    /// :``int``: The index of the monitor.
    #[getter]
    pub fn index(&self) -> Result<usize, MonitorError> {
        Ok(self.device_name()?.replace("\\\\.\\DISPLAY", "").parse()?)
    }

    /// :``int``: The refresh rate of the monitor in Hz.
    #[getter]
    pub fn refresh_rate(&self) -> Result<u32, MonitorError> {
        Ok(self.display_settings()?.dmDisplayFrequency)
    }

    /// :``str``: The monitor device name.
    #[getter]
    pub fn device_name(&self) -> Result<String, MonitorError> {
        Ok(from_wide(&self.monitor_info()?.szDevice))
    }

    /// :``str``: The device string of the monitor.
    #[getter]
    pub fn device_string(&self) -> Result<String, MonitorError> {
        let monitor_info = self.monitor_info()?;
        let mut display_device = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..DISPLAY_DEVICEW::default()
        };
        // SAFETY: `szDevice` is a NUL terminated device name inside `monitor_info`, which outlives
        // the call, and `display_device` is a live struct whose `cb` field tells the API how many
        // bytes it may write.
        let found = unsafe {
            EnumDisplayDevicesW(
                PCWSTR(monitor_info.szDevice.as_ptr()),
                0,
                &mut display_device,
                0,
            )
        };
        if !found.as_bool() {
            return Err(MonitorError::MonitorNameError);
        }
        Ok(from_wide(&display_device.DeviceString))
    }
}

impl Monitor {
    /// Return the monitor at the one-based `index`.
    pub fn from_index(index: usize) -> Result<Self, MonitorError> {
        if index < 1 {
            return Err(MonitorError::IndexError);
        }
        enumerate_monitors()?
            .get(index - 1)
            .copied()
            .ok_or(MonitorError::NotFound)
    }

    #[must_use]
    pub fn from_handle(monitor_handle: HMONITOR) -> Self {
        Self {
            monitor_handle: monitor_handle.0 as isize,
        }
    }

    #[must_use]
    pub fn as_raw_hmonitor(&self) -> HMONITOR {
        HMONITOR(self.monitor_handle as *mut c_void)
    }

    /// The monitor info block with the device name.
    fn monitor_info(&self) -> Result<MONITORINFOEXW, MonitorError> {
        let mut monitor_info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: size_of::<MONITORINFOEXW>() as u32,
                ..MONITORINFO::default()
            },
            ..MONITORINFOEXW::default()
        };
        let info_ptr = ptr::addr_of_mut!(monitor_info).cast();
        // SAFETY: GetMonitorInfoW writes at most `cbSize` bytes, which we set to the true size of
        // MONITORINFOEXW. The API expects the extended struct through a MONITORINFO pointer, and
        // `monitor_info` stays alive for the whole call. The handle validated and then only read
        let found = unsafe { GetMonitorInfoW(self.as_raw_hmonitor(), info_ptr) };
        if !found.as_bool() {
            return Err(MonitorError::MonitorInfoError);
        }
        Ok(monitor_info)
    }

    /// The current display settings of the monitor.
    fn display_settings(&self) -> Result<DEVMODEW, MonitorError> {
        let mut device_mode = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..DEVMODEW::default()
        };
        let name = HSTRING::from(self.device_name()?);
        // SAFETY: `name` is a NUL terminated wide string that outlives the call and `device_mode`
        // is a live DEVMODEW whose `dmSize` field tells the API how many bytes it may write.
        let found = unsafe {
            EnumDisplaySettingsW(
                PCWSTR(name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
            )
        };
        if !found.as_bool() {
            return Err(MonitorError::MonitorSettingsError);
        }
        Ok(device_mode)
    }
}

/// Get the primary monitor.
///
/// Returns:
///    The monitor.
///
/// Raises:
///    NotFound: No monitor contains the origin of the virtual screen.
#[pyfunction]
pub fn primary_monitor() -> Result<Monitor, MonitorError> {
    // SAFETY: The origin of the virtual screen always lies on the primary monitor.
    // MONITOR_DEFAULTTONULL makes the call return a null handle instead of a fallback if there is
    // no such monitor, which we check below.
    let monitor_handle = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONULL) };
    if monitor_handle.is_invalid() {
        return Err(MonitorError::NotFound);
    }
    Ok(Monitor::from_handle(monitor_handle))
}

// SAFETY: May only be passed to EnumDisplayMonitors together with an LPARAM holding a pointer to a
// live, unaliased Vec<Monitor>, as done in enumerate_monitors below.
unsafe extern "system" fn enum_monitors_callback(
    monitor_handle: HMONITOR,
    _: HDC,
    _: *mut RECT,
    vec: LPARAM,
) -> BOOL {
    // SAFETY: `vec` is the pointer we handed to EnumDisplayMonitors. It points to a Vec<Monitor>
    // that stays alive and is not borrowed elsewhere while the enumeration runs.
    let monitors = unsafe { &mut *(vec.0 as *mut Vec<Monitor>) };

    monitors.push(Monitor::from_handle(monitor_handle));

    TRUE
}

/// Enumerate all monitors connected to the system.
///
/// Returns:
///   The list of all monitors.
///
/// Raises:
///    WindowsError: Enumerating the monitors has failed.
#[pyfunction]
pub fn enumerate_monitors() -> Result<Vec<Monitor>, MonitorError> {
    let mut monitors: Vec<Monitor> = Vec::new();
    // SAFETY: The callback matches the signature EnumDisplayMonitors expects, and the LPARAM is a
    // pointer to `monitors`, which outlives the enumeration and is not borrowed elsewhere.
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitors_callback),
            LPARAM(ptr::addr_of_mut!(monitors) as isize),
        )
        .ok()?;
    }
    Ok(monitors)
}

// Monitor to GraphicsCaptureItem conversion
impl TryFrom<Monitor> for GraphicsCaptureItem {
    type Error = MonitorError;

    fn try_from(value: Monitor) -> Result<Self, Self::Error> {
        let interop = windows::core::factory::<Self, IGraphicsCaptureItemInterop>()?;
        // SAFETY: `interop` is a live COM interface from the WinRT factory and the handle is a
        // plain monitor handle. CreateForMonitor validates it and returns an error for an invalid
        // monitor. The returned capture item is owned by the caller.
        Ok(unsafe { interop.CreateForMonitor(value.as_raw_hmonitor())? })
    }
}
