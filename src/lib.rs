use pyo3::prelude::*;

mod platform;

/// Export the pixel_forge Rust library to Python.
///
/// The Python-facing types come from the platform selected at compile time
#[pymodule]
fn pixel_forge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(platform::enumerate_windows, m)?)?;
    m.add_function(wrap_pyfunction!(platform::foreground_window, m)?)?;
    m.add_class::<platform::Window>()?;
    m.add_function(wrap_pyfunction!(platform::primary_monitor, m)?)?;
    m.add_function(wrap_pyfunction!(platform::enumerate_monitors, m)?)?;
    m.add_class::<platform::Monitor>()?;
    m.add_class::<platform::Capture>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
