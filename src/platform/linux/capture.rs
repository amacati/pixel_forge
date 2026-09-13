use numpy::PyArray3;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

use super::monitor::Monitor;
use super::window::Window;

#[derive(FromPyObject)]
pub enum CaptureTarget {
    Monitor(Monitor),
    Window(Window),
}

/// Capture frames from a monitor or a window.
#[pyclass]
pub struct Capture;

#[pymethods]
impl Capture {
    #[new]
    fn new() -> Self {
        Self
    }

    fn start(&mut self, _target: CaptureTarget, _await_first_frame: Option<bool>) -> PyResult<()> {
        Err(PyNotImplementedError::new_err(
            "Capture in the Linux backend is not implemented yet",
        ))
    }

    #[getter]
    fn active(&self) -> bool {
        false
    }

    fn stop(&mut self) {}

    #[pyo3(name = "frame")]
    fn py_frame(&self, _py: Python) -> PyResult<Py<PyArray3<u8>>> {
        Err(PyNotImplementedError::new_err(
            "Capture in the Linux backend is not implemented yet",
        ))
    }
}
