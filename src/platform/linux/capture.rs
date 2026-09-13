use std::ptr;
use std::slice;

use numpy::ndarray::Array3;
use numpy::{PyArray3, ToPyArray};
use pyo3::prelude::*;

use x11rb::connection::Connection;
use x11rb::protocol::composite::{self, Redirect};
use x11rb::protocol::randr;
use x11rb::protocol::shm;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};
use x11rb::rust_connection::RustConnection;

use super::error::X11Error;
use super::monitor::Monitor;
use super::window::{Window, connect};

#[derive(FromPyObject)]
pub enum CaptureTarget {
    Monitor(Monitor),
    Window(Window),
}

// Capture target variants require separate handling.
enum Target {
    Window { pixmap: u32, window: u32, owns_redirect: bool },
    Monitor,
}

// Session holds all resources for a running capture
struct Session {
    conn: RustConnection,
    drawable: u32,  // window's off-screen pixmap, or the root window for a monitor
    x: i16,  // region origin within the drawable. (0, 0) for a window, monitor origin otherwise
    y: i16,
    width: u16,
    height: u16,
    shmseg: u32,  // shared memory segment ID of the X server
    addr: *mut libc::c_void,
    size: usize,
    target: Target,
}

/// Capture frames from a window or a monitor.
#[pyclass(unsendable)]
pub struct Capture {
    session: Option<Session>,
}

#[pymethods]
impl Capture {
    #[new]
    fn new() -> Self {
        Self { session: None }
    }

    /// Set up the capture. A window gets redirected through Composite and its off-screen pixmap
    /// named, so it captures correctly even when occluded. A monitor reads the root window at the
    /// monitor's region. Frames are then read on demand in :meth:`frame`.
    #[pyo3(signature = (capture_target, await_first_frame=None))]
    fn start(&mut self, capture_target: CaptureTarget, await_first_frame: Option<bool>) -> Result<(), X11Error> {
        let _ = await_first_frame; // Capture is always synchronous on Linux
        self.stop();
        self.session = Some(match capture_target {
            CaptureTarget::Window(window) => start_window(window.window)?,
            CaptureTarget::Monitor(monitor) => start_monitor(monitor.monitor_index())?,
        });
        Ok(())
    }

    /// :``bool``: True if a capture is running.
    #[getter]
    fn active(&self) -> bool {
        self.session.is_some()
    }

    /// Stop the capture and release the pixmap, shared memory, and redirection.
    fn stop(&mut self) {
        let Some(s) = self.session.take() else {
            return;
        };
        let _ = shm::detach(&s.conn, s.shmseg);
        // SAFETY: We allocated s.addr with shmat, and are the only owner of it, so it must be safe
        // to detach it here.
        unsafe { libc::shmdt(s.addr) };
        if let Target::Window { pixmap, window, owns_redirect } = s.target {
            let _ = s.conn.free_pixmap(pixmap);
            if owns_redirect {
                let _ = composite::unredirect_window(&s.conn, window, Redirect::AUTOMATIC);
            }
        }
        let _ = s.conn.flush();
    }

    /// Grab the current contents of the target and return them as an [h, w, 4] RGBA array.
    #[pyo3(name = "frame")]
    fn py_frame(&self, py: Python) -> Result<Py<PyArray3<u8>>, X11Error> {
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| X11Error::Other("Capture is not running".into()))?;
        shm::get_image(
            &s.conn,
            s.drawable,
            s.x,
            s.y,
            s.width,
            s.height,
            u32::MAX,
            ImageFormat::Z_PIXMAP.into(),
            s.shmseg,
            0,
        )?
        .reply()?;

        // X has written the frame into shared memory as BGRX, so we swap to RGBA and set alpha to full.
        // SAFETY: s.addr points to the shared segment of s.size bytes and stays attached for the
        // Session's life.
        let raw = unsafe { slice::from_raw_parts(s.addr.cast::<u8>(), s.size) };
        let mut rgba = vec![0u8; s.size];
        for (dst, src) in rgba.chunks_exact_mut(4).zip(raw.chunks_exact(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = 255;
        }
        let array = Array3::from_shape_vec((s.height as usize, s.width as usize, 4), rgba)
            .map_err(|e| X11Error::Other(e.to_string()))?;
        Ok(array.to_pyarray(py).unbind())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn start_window(window: u32) -> Result<Session, X11Error> {
    let (conn, _) = connect()?;

    // Read the window geometry and reserve the pixmap id before redirecting
    let geometry = conn.get_geometry(window)?.reply()?;
    let (width, height) = (geometry.width, geometry.height);
    let pixmap = conn.generate_id()?;

    // X does not render occluded windows, so we use a pixmap of the same size to force it to render
    // the full window even when it's covered. The redirect fails if we already redirected with
    // another client.
    let owns_redirect = composite::redirect_window(&conn, window, Redirect::AUTOMATIC)?
        .check()
        .is_ok();

    // Name the pixmap and attach a shared memory segment for fast reads.
    // TODO: A window resize allocates a new pixmap, so frames go stale until the capture is
    // restarted. We may want to use ConfigNotify events to detect resizes and fix that
    let size = width as usize * height as usize * 4;
    let setup = composite::name_window_pixmap(&conn, window, pixmap)
        .map_err(X11Error::from)
        .and_then(|cookie| cookie.check().map_err(X11Error::from))
        .and_then(|()| attach_shm(&conn, size));

    match setup {
        Ok((shmseg, addr)) => Ok(Session {
            conn,
            drawable: pixmap,
            x: 0,
            y: 0,
            width,
            height,
            shmseg,
            addr,
            size,
            target: Target::Window { pixmap, window, owns_redirect },
        }),
        Err(e) => {
            let _ = conn.free_pixmap(pixmap);
            if owns_redirect {
                let _ = composite::unredirect_window(&conn, window, Redirect::AUTOMATIC);
            }
            let _ = conn.flush();
            Err(e)
        }
    }
}

fn start_monitor(index: usize) -> Result<Session, X11Error> {
    // Reads the root window at the monitor's region
    let (conn, screen) = connect()?;
    let root = conn.setup().roots[screen].root;
    let mut monitors = randr::get_monitors(&conn, root, true)?.reply()?.monitors;
    if index < 1 || index > monitors.len() {
        return Err(X11Error::NoMonitor);
    }
    let monitor = monitors.swap_remove(index - 1);
    let size = monitor.width as usize * monitor.height as usize * 4;
    let (shmseg, addr) = attach_shm(&conn, size)?;

    Ok(Session {
        conn,
        drawable: root,
        x: monitor.x,
        y: monitor.y,
        width: monitor.width,
        height: monitor.height,
        shmseg,
        addr,
        size,
        target: Target::Monitor,
    })
}

// Create a shared memory segment of `size` bytes and attach it to the process and X server.
// Cleans up the segment on any failure.
fn attach_shm(conn: &RustConnection, size: usize) -> Result<(u32, *mut libc::c_void), X11Error> {
    // SAFETY: IPC_PRIVATE creates a segment of `size` bytes. The flags are a valid mode
    let shmid = unsafe { libc::shmget(libc::IPC_PRIVATE, size, libc::IPC_CREAT | 0o600) };
    if shmid < 0 {
        return Err(X11Error::Other("shmget failed".into()));
    }

    // SAFETY: we just created shmid. A null address lets the kernel place the mapping. The result
    // is validated below before we use it.
    let addr = unsafe { libc::shmat(shmid, ptr::null(), 0) };
    if addr == usize::MAX as *mut libc::c_void {
        // SAFETY: shmid is the valid, unattached segment. IPC_RMID frees it
        unsafe { libc::shmctl(shmid, libc::IPC_RMID, ptr::null_mut()) };
        return Err(X11Error::Other("shmat failed".into()));
    }

    let attached = conn.generate_id().map_err(X11Error::from).and_then(|shmseg| {
        shm::attach(conn, shmseg, shmid as u32, false)
            .map_err(X11Error::from)
            .and_then(|cookie| cookie.check().map_err(X11Error::from))
            .map(|()| shmseg)
    });

    match attached {
        Ok(shmseg) => {
            // We have attached shmid to the X server, so we can mark it free on our side. The X
            // server keeps it alive until we detach it.
            // SAFETY: shmid has been checked to be valid above
            unsafe { libc::shmctl(shmid, libc::IPC_RMID, ptr::null_mut()) };
            Ok((shmseg, addr))
        }
        Err(e) => {
            // SAFETY: addr is our mapping from shmat. We detach it and free i. Both ids/pointers
            // are valid and released only once.
            unsafe {
                libc::shmdt(addr);
                libc::shmctl(shmid, libc::IPC_RMID, ptr::null_mut());
            }
            Err(e)
        }
    }
}
