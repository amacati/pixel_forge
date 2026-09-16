use pyo3::prelude::*;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConfigureWindowAux, ConnectionExt, InputFocus, MapState, StackMode,
};
use x11rb::rust_connection::RustConnection;

use super::error::X11Error;

/// Open a connection to the DISPLAY X server.
pub(super) fn connect() -> Result<(RustConnection, usize), X11Error> {
    Ok(x11rb::connect(None)?)
}

fn intern(conn: &RustConnection, name: &[u8]) -> Result<u32, X11Error> {
    Ok(conn.intern_atom(false, name)?.reply()?.atom)
}

/// The tracked windows for the window manager
fn client_list(conn: &RustConnection, root: u32) -> Result<Vec<u32>, X11Error> {
    let atom = intern(conn, b"_NET_CLIENT_LIST")?;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, u32::MAX)?
        .reply()?;
    Ok(reply.value32().map(Iterator::collect).unwrap_or_default())
}

fn title(conn: &RustConnection, window: u32) -> Result<Option<String>, X11Error> {
    let net_wm_name = intern(conn, b"_NET_WM_NAME")?; // preferred property
    let utf8 = intern(conn, b"UTF8_STRING")?;
    let reply = conn
        .get_property(false, window, net_wm_name, utf8, 0, u32::MAX)?
        .reply()?;
    if !reply.value.is_empty() {
        return Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()));
    }
    // Fallback to legacy WM_NAME if _NET_WM_NAME didn't work
    let reply = conn
        .get_property(
            false,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            0,
            u32::MAX,
        )?
        .reply()?;
    if reply.value.is_empty() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()))
}

/// Window abstraction for X11.
///
/// Windows are capture targets for the :class:`.Capture` class.
#[pyclass(from_py_object)]
#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub(super) window: u32,
}

#[pymethods]
impl Window {
    /// Create a window from its title
    #[new]
    fn new(name: &str) -> Result<Self, X11Error> {
        let (conn, screen) = connect()?;
        let root = conn.setup().roots[screen].root;
        for window in client_list(&conn, root)? {
            if title(&conn, window)?.as_deref() == Some(name) {
                return Ok(Self { window });
            }
        }
        Err(X11Error::WindowNotFound(name.to_string()))
    }

    /// :``bool``: True if the window is still mapped and viewable, else False.
    #[getter]
    fn valid(&self) -> bool {
        let check = || -> Result<bool, X11Error> {
            let (conn, _) = connect()?;
            let attrs = conn.get_window_attributes(self.window)?.reply()?;
            Ok(attrs.map_state == MapState::VIEWABLE)
        };
        check().unwrap_or(false)
    }

    /// :``str``: The window title.
    #[getter]
    fn name(&self) -> Result<String, X11Error> {
        let (conn, _) = connect()?;
        Ok(title(&conn, self.window)?.unwrap_or_default())
    }

    /// Raise the window and give it the input focus.
    fn focus(&self) -> Result<(), X11Error> {
        let (conn, _) = connect()?;
        conn.configure_window(
            self.window,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )?;
        conn.set_input_focus(InputFocus::PARENT, self.window, x11rb::CURRENT_TIME)?;
        conn.flush()?;
        Ok(())
    }

    /// :``bool``: True if the window currently holds the input focus, else False.
    #[getter]
    fn focused(&self) -> Result<bool, X11Error> {
        let (conn, _) = connect()?;
        Ok(conn.get_input_focus()?.reply()?.focus == self.window)
    }
}

/// List all tracked windows.
#[pyfunction]
pub fn enumerate_windows() -> Result<Vec<Window>, X11Error> {
    let (conn, screen) = connect()?;
    let root = conn.setup().roots[screen].root;
    Ok(client_list(&conn, root)?
        .into_iter()
        .map(|window| Window { window })
        .collect())
}

/// Get the currently active window.
#[pyfunction]
pub fn foreground_window() -> Result<Window, X11Error> {
    let (conn, screen) = connect()?;
    let root = conn.setup().roots[screen].root;
    let atom = intern(&conn, b"_NET_ACTIVE_WINDOW")?;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)?
        .reply()?;
    match reply
        .value32()
        .and_then(|mut it| it.next())
        .filter(|&w| w != 0)
    {
        Some(window) => Ok(Window { window }),
        None => Err(X11Error::NoActiveWindow),
    }
}
