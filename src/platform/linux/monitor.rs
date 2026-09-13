use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

fn pending<T>() -> PyResult<T> {
    Err(PyNotImplementedError::new_err(
        "Monitor support in the Linux backend is not implemented yet",
    ))
}

/// Monitor abstraction for X11.
#[pyclass(from_py_object)]
#[derive(Clone, Copy, Debug)]
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
