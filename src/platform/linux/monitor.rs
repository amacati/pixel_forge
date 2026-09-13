use pyo3::prelude::*;

use x11rb::connection::Connection;
use x11rb::protocol::randr::{self, ConnectionExt as _, MonitorInfo};
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

use super::error::X11Error;
use super::window::connect;

/// The vector of current RandR monitors.
fn monitors(conn: &RustConnection, root: u32) -> Result<Vec<MonitorInfo>, X11Error> {
    Ok(randr::get_monitors(conn, root, true)?.reply()?.monitors)
}

/// Monitor abstraction for X11.
///
/// Monitors are indexed from one, matching the Windows backend. A monitor can be used as a capture
/// target for the :class:`.Capture` class.
#[pyclass(from_py_object)]
#[derive(Clone, Copy, Debug)]
pub struct Monitor {
    index: usize,
}

impl Monitor {
    pub(super) const fn from_index(index: usize) -> Self {
        Self { index }
    }

    pub(super) const fn monitor_index(&self) -> usize {
        self.index
    }

    /// Look this monitor up in the current monitor list.
    fn info(&self) -> Result<(RustConnection, u32, MonitorInfo), X11Error> {
        let (conn, screen) = connect()?;
        let root = conn.setup().roots[screen].root;
        let mut list = monitors(&conn, root)?;
        if self.index < 1 || self.index > list.len() {
            return Err(X11Error::NoMonitor);
        }
        let monitor = list.swap_remove(self.index - 1);
        Ok((conn, root, monitor))
    }
}

#[pymethods]
impl Monitor {
    #[new]
    #[pyo3(signature = (id=None))]
    fn new(id: Option<usize>) -> Result<Self, X11Error> {
        match id {
            Some(index) if index >= 1 => Ok(Self { index }),
            Some(_) => Err(X11Error::MonitorIndex),
            None => primary_monitor(),
        }
    }

    /// :``int``: The pixel width of the monitor.
    #[getter]
    fn width(&self) -> Result<u32, X11Error> {
        Ok(u32::from(self.info()?.2.width))
    }

    /// :``int``: The pixel height of the monitor.
    #[getter]
    fn height(&self) -> Result<u32, X11Error> {
        Ok(u32::from(self.info()?.2.height))
    }

    /// :``int``: The one-based index of the monitor.
    #[getter]
    fn index(&self) -> usize {
        self.index
    }

    /// :``int``: The refresh rate of the monitor in Hz or zero if it is unknown.
    #[getter]
    fn refresh_rate(&self) -> Result<u32, X11Error> {
        let (conn, root, monitor) = self.info()?;
        let Some(&output) = monitor.outputs.first() else {
            return Ok(0);
        };
        let resources = conn.randr_get_screen_resources_current(root)?.reply()?;
        let output_info = conn
            .randr_get_output_info(output, resources.config_timestamp)?
            .reply()?;
        if output_info.crtc == 0 {
            return Ok(0);
        }
        let crtc = conn
            .randr_get_crtc_info(output_info.crtc, resources.config_timestamp)?
            .reply()?;
        let mode = resources.modes.iter().find(|m| m.id == crtc.mode);
        match mode {
            Some(m) if m.htotal != 0 && m.vtotal != 0 => {
                let rate = f64::from(m.dot_clock) / (f64::from(m.htotal) * f64::from(m.vtotal));
                Ok(rate.round() as u32)
            }
            _ => Ok(0),
        }
    }

    /// :``str``: The name of the monitor, for example ``HDMI-1``.
    #[getter]
    fn device_name(&self) -> Result<String, X11Error> {
        let (conn, _, monitor) = self.info()?;
        let name = conn.get_atom_name(monitor.name)?.reply()?.name;
        Ok(String::from_utf8_lossy(&name).into_owned())
    }

    /// :``str``: A description of the monitor.
    #[getter]
    fn device_string(&self) -> Result<String, X11Error> {
        self.device_name()
    }
}

/// Get the primary monitor, or the first monitor if there is no primary monitor.
#[pyfunction]
pub fn primary_monitor() -> Result<Monitor, X11Error> {
    let (conn, screen) = connect()?;
    let root = conn.setup().roots[screen].root;
    let list = monitors(&conn, root)?;
    if list.is_empty() {
        return Err(X11Error::NoMonitor);
    }
    let index = list.iter().position(|m| m.primary).unwrap_or(0) + 1;
    Ok(Monitor { index })
}

/// Enumerate all monitors.
#[pyfunction]
pub fn enumerate_monitors() -> Result<Vec<Monitor>, X11Error> {
    let (conn, screen) = connect()?;
    let root = conn.setup().roots[screen].root;
    let count = monitors(&conn, root)?.len();
    Ok((1..=count).map(Monitor::from_index).collect())
}
