//! Linux (X11) backend.
//!
//! Raises `NotImplementedError` for all functions, as the Linux backend is not implemented yet.

use numpy::PyArray3;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

fn pending<T>() -> PyResult<T> {
    Err(PyNotImplementedError::new_err(
        "The Linux backend is not implemented yet",
    ))
}

/// Window abstraction for the X11 windowing system.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Window;

#[pymethods]
impl Window {
    #[new]
    fn new(_name: &str) -> PyResult<Self> {
        pending()
    }

    #[getter]
    fn valid(&self) -> bool {
        false
    }

    #[getter]
    fn name(&self) -> PyResult<String> {
        pending()
    }
}

#[pyfunction]
pub fn enumerate_windows() -> PyResult<Vec<Window>> {
    pending()
}

#[pyfunction]
pub fn foreground_window() -> PyResult<Window> {
    pending()
}

/// Monitor abstraction for the X11 windowing system.
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Monitor;

#[pymethods]
impl Monitor {
    #[new]
    fn new(_id: Option<usize>) -> PyResult<Self> {
        pending()
    }

    #[getter]
    fn width(&self) -> PyResult<u32> {
        pending()
    }

    #[getter]
    fn height(&self) -> PyResult<u32> {
        pending()
    }

    #[getter]
    fn index(&self) -> PyResult<usize> {
        pending()
    }

    #[getter]
    fn refresh_rate(&self) -> PyResult<u32> {
        pending()
    }

    #[getter]
    fn device_name(&self) -> PyResult<String> {
        pending()
    }

    #[getter]
    fn device_string(&self) -> PyResult<String> {
        pending()
    }
}

#[pyfunction]
pub fn primary_monitor() -> PyResult<Monitor> {
    pending()
}

#[pyfunction]
pub fn enumerate_monitors() -> PyResult<Vec<Monitor>> {
    pending()
}

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
        pending()
    }

    #[getter]
    fn active(&self) -> bool {
        false
    }

    fn stop(&mut self) {}

    #[pyo3(name = "frame")]
    fn py_frame(&self, _py: Python) -> PyResult<Py<PyArray3<u8>>> {
        pending()
    }
}
