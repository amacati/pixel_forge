//! Windows backend, built on the Windows Graphics Capture API.

pub mod capture;
mod direct_x;
mod frame;
pub mod monitor;
pub mod window;

pub use capture::Capture;
pub use monitor::{Monitor, enumerate_monitors, primary_monitor};
pub use window::{Window, enumerate_windows, foreground_window};
