// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::mem;
use std::num::ParseIntError;
use std::string::FromUtf16Error;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::{HSTRING, PCWSTR};
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Foundation::{BOOL, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW,
    MonitorFromPoint, DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFO,
    MONITORINFOEXW, MONITOR_DEFAULTTONULL,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

#[derive(thiserror::Error, Debug)]
pub enum MonitorError {
    #[error("Failed to find monitor")]
    NotFound,
    #[error("Failed to find monitor name")]
    NameNotFound,
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
    #[error("Failed to convert windows string: {0}")]
    MonitorStringError(#[from] FromUtf16Error),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<MonitorError> for PyErr {
    fn from(error: MonitorError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

/// Monitor(id: int | None = None) -> Monitor
/// Monitor class for the Windows operating system.
///
/// Monitor can be used as capture target for the :class:`.Capture` class. It also provides some
/// convenience methods to get information about the monitor.
///
/// Args:
///    id: The index of the monitor. If None, the primary monitor is used.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[pyclass]
pub struct Monitor {
    monitor_handle: HMONITOR,
}

#[pymethods]
impl Monitor {
    /// new(id: int | None = None) -> Monitor
    ///
    /// Create a :class:`.Monitor` instance.
    ///
    /// Args:
    ///    id: The monitor ID. If None, the primary monitor is used.
    #[new]
    pub fn new(id: Option<usize>) -> Self {
        match id {
            Some(id) => Monitor::from_index(id).unwrap(),
            None => primary_monitor().unwrap(),
        }
    }

    /// :``int``: The pixel width of the monitor.
    #[getter]
    pub fn width(&self) -> Result<u32, MonitorError> {
        let mut device_mode = DEVMODEW {
            dmSize: u16::try_from(mem::size_of::<DEVMODEW>()).unwrap(),
            ..DEVMODEW::default()
        };
        let name = HSTRING::from(self.device_name()?);
        if unsafe {
            !EnumDisplaySettingsW(
                PCWSTR(name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorSettingsError);
        }

        Ok(device_mode.dmPelsWidth)
    }

    /// :``int``: The pixel height of the monitor.
    #[getter]
    pub fn height(&self) -> Result<u32, MonitorError> {
        let mut device_mode = DEVMODEW {
            dmSize: u16::try_from(mem::size_of::<DEVMODEW>()).unwrap(),
            ..DEVMODEW::default()
        };
        let name = HSTRING::from(self.device_name()?);
        if unsafe {
            !EnumDisplaySettingsW(
                PCWSTR(name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorSettingsError);
        }

        Ok(device_mode.dmPelsHeight)
    }

    /// :``int``: The index of the monitor.
    #[getter]
    pub fn index(&self) -> Result<usize, MonitorError> {
        let device_name = self.device_name()?;
        Ok(device_name.replace("\\\\.\\DISPLAY", "").parse()?)
    }

    /// :``int``: The refresh rate of the monitor in Hz.
    #[getter]
    pub fn refresh_rate(&self) -> Result<u32, MonitorError> {
        let mut device_mode = DEVMODEW {
            dmSize: u16::try_from(mem::size_of::<DEVMODEW>()).unwrap(),
            ..DEVMODEW::default()
        };
        let name = HSTRING::from(self.device_name()?);
        if unsafe {
            !EnumDisplaySettingsW(
                PCWSTR(name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut device_mode,
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorSettingsError);
        }

        Ok(device_mode.dmDisplayFrequency)
    }

    /// :``str``: The monitor device name.
    #[getter]
    pub fn device_name(&self) -> Result<String, MonitorError> {
        let mut monitor_info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: u32::try_from(mem::size_of::<MONITORINFOEXW>()).unwrap(),
                rcMonitor: RECT::default(),
                rcWork: RECT::default(),
                dwFlags: 0,
            },
            szDevice: [0; 32],
        };
        if unsafe {
            !GetMonitorInfoW(
                self.as_raw_hmonitor(),
                std::ptr::addr_of_mut!(monitor_info).cast(),
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorInfoError);
        }

        let device_name = String::from_utf16(
            &monitor_info
                .szDevice
                .as_slice()
                .iter()
                .take_while(|ch| **ch != 0x0000)
                .copied()
                .collect::<Vec<u16>>(),
        )?;

        Ok(device_name)
    }

    /// :``str``: The device string of the monitor.
    #[getter]
    pub fn device_string(&self) -> Result<String, MonitorError> {
        let mut monitor_info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: u32::try_from(mem::size_of::<MONITORINFOEXW>()).unwrap(),
                rcMonitor: RECT::default(),
                rcWork: RECT::default(),
                dwFlags: 0,
            },
            szDevice: [0; 32],
        };
        if unsafe {
            !GetMonitorInfoW(
                self.as_raw_hmonitor(),
                std::ptr::addr_of_mut!(monitor_info).cast(),
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorInfoError);
        }

        let mut display_device = DISPLAY_DEVICEW {
            cb: u32::try_from(mem::size_of::<DISPLAY_DEVICEW>()).unwrap(),
            DeviceName: [0; 32],
            DeviceString: [0; 128],
            StateFlags: 0,
            DeviceID: [0; 128],
            DeviceKey: [0; 128],
        };

        if unsafe {
            !EnumDisplayDevicesW(
                PCWSTR::from_raw(monitor_info.szDevice.as_mut_ptr()),
                0,
                &mut display_device,
                0,
            )
            .as_bool()
        } {
            return Err(MonitorError::MonitorNameError);
        }

        let device_string = String::from_utf16(
            &display_device
                .DeviceString
                .as_slice()
                .iter()
                .take_while(|ch| **ch != 0x0000)
                .copied()
                .collect::<Vec<u16>>(),
        )?;

        Ok(device_string)
    }
}

impl Monitor {
    /// Return the monitor at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the monitor to retrieve. The index starts from 1.
    ///
    /// # Errors
    ///
    /// `MonitorError::IndexError`: The index is less than 1.
    /// `MonitorError::NotFound`: The monitor at the specified index is not found.
    pub fn from_index(index: usize) -> Result<Self, MonitorError> {
        if index < 1 {
            return Err(MonitorError::IndexError);
        }

        let monitor = enumerate_monitors()?;
        let monitor = match monitor.get(index - 1) {
            Some(monitor) => *monitor,
            None => return Err(MonitorError::NotFound),
        };

        Ok(monitor)
    }

    /// Create a `Monitor` instance from a raw HMONITOR.
    ///
    /// # Arguments
    ///
    /// * `monitor_handle` - The raw HMONITOR.
    #[must_use]
    pub const fn from_handle(monitor_handle: HMONITOR) -> Self {
        Self { monitor_handle }
    }

    /// Returns the raw HMONITOR of the monitor.
    #[must_use]
    pub const fn as_raw_hmonitor(&self) -> HMONITOR {
        self.monitor_handle
    }
}

/// primary_monitor() -> Monitor
///
/// Get the primary monitor.
///
/// Returns:
///    The monitor.
#[pyfunction]
pub fn primary_monitor() -> Result<Monitor, MonitorError> {
    let point = POINT { x: 0, y: 0 };
    let monitor_handle = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONULL) };

    if monitor_handle.is_invalid() {
        return Err(MonitorError::NotFound);
    }

    Ok(Monitor { monitor_handle })
}

// Callback Used For Enumerating All Monitors
unsafe extern "system" fn enum_monitors_callback(
    monitor_handle: HMONITOR,
    _: HDC,
    _: *mut RECT,
    vec: LPARAM,
) -> BOOL {
    let monitors = &mut *(vec.0 as *mut Vec<Monitor>);

    monitors.push(Monitor { monitor_handle });

    TRUE
}

/// enumerate_monitors() -> list[Monitor]
///
/// Enumerate all monitors connected to the system.
///
/// Returns:
///   The list of all monitors.
#[pyfunction]
pub fn enumerate_monitors() -> Result<Vec<Monitor>, MonitorError> {
    let mut monitors: Vec<Monitor> = Vec::new();

    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitors_callback),
            LPARAM(std::ptr::addr_of_mut!(monitors) as isize),
        )
        .ok()?;
    };

    Ok(monitors)
}

// Implements TryFrom For Monitor To Convert It To GraphicsCaptureItem
impl TryFrom<Monitor> for GraphicsCaptureItem {
    type Error = MonitorError;

    fn try_from(value: Monitor) -> Result<Self, Self::Error> {
        let monitor = value.as_raw_hmonitor();

        let interop = windows::core::factory::<Self, IGraphicsCaptureItemInterop>()?;
        Ok(unsafe { interop.CreateForMonitor(monitor)? })
    }
}
