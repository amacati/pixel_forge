//! Each OS provides the same set of Python-facing types and functions. The active backend is
//! selected at compile time and re-exported here, so the module registration and the Python API are
//! identical on every platform.

use numpy::{PyArray3, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// The array a frame is written into: either the caller's `out` buffer, checked against the frame
/// dimensions, or a newly allocated one.
pub(crate) fn destination<'py>(
    py: Python<'py>,
    out: Option<Bound<'py, PyArray3<u8>>>,
    height: usize,
    width: usize,
) -> PyResult<Bound<'py, PyArray3<u8>>> {
    let Some(out) = out else {
        // SAFETY: left uninitialised because the caller overwrites every byte of it.
        return Ok(unsafe { PyArray3::<u8>::new(py, [height, width, 4], false) });
    };
    if out.shape() != [height, width, 4] {
        let shape = out.shape();
        return Err(PyValueError::new_err(format!(
            "out has shape {shape:?}, expected [{height}, {width}, 4]"
        )));
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::{
    enumerate_monitors, enumerate_windows, foreground_window, primary_monitor, Capture, Monitor,
    Window,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{
    enumerate_monitors, enumerate_windows, foreground_window, primary_monitor, Capture, Monitor,
    Window,
};
