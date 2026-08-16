//! Windows backend, built on the Windows Graphics Capture API.

pub mod capture;
mod capture_utils;
mod direct_x;
mod frame;
pub mod monitor;
pub mod window;

pub use capture::Capture;
pub use monitor::{enumerate_monitors, primary_monitor, Monitor};
pub use window::{enumerate_windows, foreground_window, Window};
