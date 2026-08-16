//! Each OS provides the same set of Python-facing types and functions. The active backend is
//! selected at compile time and re-exported here, so the module registration and the Python API are
//! identical on every platform.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
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
