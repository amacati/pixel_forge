//! Linux (X11) backend.
//!
//! Window lookup and capture go through the X server over an XCB connection.
//! TODO: The monitor path is not implemented yet.

mod capture;
mod error;
mod monitor;
mod window;

pub use capture::Capture;
pub use monitor::{Monitor, enumerate_monitors, primary_monitor};
pub use window::{Window, enumerate_windows, foreground_window};
