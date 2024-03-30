use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

pub mod monitor;
pub mod window;

/// Export the pixel_forge Rust library to Python.
#[pymodule]
fn pixel_forge_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(window::enumerate_windows, m)?)?;
    m.add_function(wrap_pyfunction!(window::foreground_window, m)?)?;
    m.add_class::<window::Window>()?;

    Ok(())
}
