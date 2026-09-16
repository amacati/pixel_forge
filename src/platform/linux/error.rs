use pyo3::PyErr;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

/// Errors raised by the X11 backend.
#[derive(thiserror::Error, Debug)]
pub enum X11Error {
    #[error("Failed to connect to the X server: {0}")]
    Connect(#[from] x11rb::errors::ConnectError),
    #[error("X11 connection error: {0}")]
    Connection(#[from] x11rb::errors::ConnectionError),
    #[error("X11 reply error: {0}")]
    Reply(#[from] x11rb::errors::ReplyError),
    #[error("X11 request error: {0}")]
    ReplyOrId(#[from] x11rb::errors::ReplyOrIdError),
    #[error("Window '{0}' not found")]
    WindowNotFound(String),
    #[error("No active window")]
    NoActiveWindow,
    #[error("No monitor found")]
    NoMonitor,
    #[error("Monitor index must be one or greater")]
    MonitorIndex,
    #[error("{0}")]
    Other(String),
    #[error("Frame buffer is not contiguous: {0}")]
    NotContiguous(#[from] numpy::AsSliceError),
    #[error(transparent)]
    Python(#[from] PyErr),
}

impl From<X11Error> for PyErr {
    fn from(error: X11Error) -> PyErr {
        match error {
            X11Error::Python(err) => err,
            e @ X11Error::NotContiguous(_) => PyValueError::new_err(e.to_string()),
            e => PyRuntimeError::new_err(e.to_string()),
        }
    }
}
