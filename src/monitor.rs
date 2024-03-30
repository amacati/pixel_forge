use std::{mem, num::ParseIntError, string::FromUtf16Error};

use windows::{
    core::{HSTRING, PCWSTR},
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Devices::Display::{
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
            DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
            DISPLAYCONFIG_TARGET_DEVICE_NAME_FLAGS, DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY,
            QDC_ONLY_ACTIVE_PATHS,
        },
        Foundation::{BOOL, LPARAM, POINT, RECT, TRUE},
        Graphics::Gdi::{
            EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW,
            MonitorFromPoint, DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR,
            MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONULL,
        },
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};

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
    FailedToParseMonitorIndex(#[from] ParseIntError),
    #[error("Failed to convert windows string: {0}")]
    FailedToConvertWindowsString(#[from] FromUtf16Error),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

/// Monitor device for the Windows operating system.
///
/// # Example
/// ```no_run
/// use pixel_forge::monitor::Monitor;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let monitor = Monitor::primary()?;
///     println!("Primary Monitor: {}", monitor.name()?);
///
///     Ok(())
/// }
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Monitor {
    monitor_handle: HMONITOR,
}

impl Monitor {
    /// Return the primary monitor.
    ///
    /// # Errors
    ///
    /// `MonitorError::NotFound`: There is no primary monitor.
    pub fn primary() -> Result<Self, MonitorError> {
        let point = POINT { x: 0, y: 0 };
        let monitor_handle = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONULL) };

        if monitor_handle.is_invalid() {
            return Err(MonitorError::NotFound);
        }

        Ok(Self { monitor_handle })
    }

    /// Returns the monitor at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the monitor to retrieve. The index starts from 1.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError::IndexError` if the index is less than 1.
    /// Returns an `MonitorError::NotFound` if the monitor at the specified index is not found.
    pub fn from_index(index: usize) -> Result<Self, MonitorError> {
        if index < 1 {
            return Err(MonitorError::IndexError);
        }

        let monitor = Self::enumerate()?;
        let monitor = match monitor.get(index - 1) {
            Some(monitor) => *monitor,
            None => return Err(MonitorError::NotFound),
        };

        Ok(monitor)
    }

    /// Returns the index of the monitor.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor index.
    pub fn index(&self) -> Result<usize, MonitorError> {
        let device_name = self.device_name()?;
        Ok(device_name.replace("\\\\.\\DISPLAY", "").parse()?)
    }

    /// Returns the name of the monitor.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor name.
    pub fn name(&self) -> Result<String, MonitorError> {
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

        let mut number_of_paths = 0;
        let mut number_of_modes = 0;
        unsafe {
            GetDisplayConfigBufferSizes(
                QDC_ONLY_ACTIVE_PATHS,
                &mut number_of_paths,
                &mut number_of_modes,
            )
            .ok()?;
        };

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); number_of_paths as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); number_of_modes as usize];
        unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut number_of_paths,
                paths.as_mut_ptr(),
                &mut number_of_modes,
                modes.as_mut_ptr(),
                None,
            )
        }
        .ok()?;

        for path in paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: u32::try_from(mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>())
                        .unwrap(),
                    adapterId: path.sourceInfo.adapterId,
                    id: path.sourceInfo.id,
                },
                viewGdiDeviceName: [0; 32],
            };

            let device_name = self.device_name()?;
            let view_gdi_device_name = String::from_utf16(
                &monitor_info
                    .szDevice
                    .as_slice()
                    .iter()
                    .take_while(|ch| **ch != 0x0000)
                    .copied()
                    .collect::<Vec<u16>>(),
            )?;

            if unsafe { DisplayConfigGetDeviceInfo(&mut source.header) } == 0
                && device_name == view_gdi_device_name
            {
                let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                    header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                        size: u32::try_from(mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>())
                            .unwrap(),
                        adapterId: path.sourceInfo.adapterId,
                        id: path.targetInfo.id,
                    },
                    flags: DISPLAYCONFIG_TARGET_DEVICE_NAME_FLAGS::default(),
                    outputTechnology: DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY::default(),
                    edidManufactureId: 0,
                    edidProductCodeId: 0,
                    connectorInstance: 0,
                    monitorFriendlyDeviceName: [0; 64],
                    monitorDevicePath: [0; 128],
                };

                if unsafe { DisplayConfigGetDeviceInfo(&mut target.header) } == 0 {
                    let name = String::from_utf16(
                        &target
                            .monitorFriendlyDeviceName
                            .as_slice()
                            .iter()
                            .take_while(|ch| **ch != 0x0000)
                            .copied()
                            .collect::<Vec<u16>>(),
                    )?;
                    return Ok(name);
                }

                return Err(MonitorError::MonitorInfoError);
            }
        }

        Err(MonitorError::NameNotFound)
    }

    /// Returns the device name of the monitor.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor device name.
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

    /// Returns the device string of the monitor.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor device string.
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

    /// Returns the refresh rate of the monitor in hertz.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor refresh rate.
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

    /// Returns the width of the monitor in pixels.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor width.
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

    /// Returns the height of the monitor in pixels.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error retrieving the monitor height.
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

    /// Returns a list of all monitors.
    ///
    /// # Errors
    ///
    /// Returns an `MonitorError` if there is an error enumerating the monitors.
    pub fn enumerate() -> Result<Vec<Self>, MonitorError> {
        let mut monitors: Vec<Self> = Vec::new();

        unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(Self::enum_monitors_callback),
                LPARAM(std::ptr::addr_of_mut!(monitors) as isize),
            )
            .ok()?;
        };

        Ok(monitors)
    }

    /// Creates a `Monitor` instance from a raw HMONITOR.
    ///
    /// # Arguments
    ///
    /// * `monitor` - The raw HMONITOR.
    #[must_use]
    pub const fn from_raw_hmonitor(monitor_handle: HMONITOR) -> Self {
        Self { monitor_handle }
    }

    /// Returns the raw HMONITOR of the monitor.
    #[must_use]
    pub const fn as_raw_hmonitor(&self) -> HMONITOR {
        self.monitor_handle
    }

    // Callback Used For Enumerating All Monitors
    unsafe extern "system" fn enum_monitors_callback(
        monitor_handle: HMONITOR,
        _: HDC,
        _: *mut RECT,
        vec: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(vec.0 as *mut Vec<Self>);

        monitors.push(Self { monitor_handle });

        TRUE
    }
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
