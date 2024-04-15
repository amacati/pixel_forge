use pyo3::prelude::*;

use windows::Graphics::Capture::GraphicsCaptureItem;

use crate::monitor::Monitor;
use crate::window::Window;

// We define a capture target as either a monitor or a window. Pyo3 does not allow functions
// generics, so we have to use an enum to represent the two types of capture sources that we can
// pass to Capture::start. We also define the TryInto trait for CaptureTarget to convert it into a
// GraphicsCaptureItem, which is what we ultimately need to start capturing frames.
#[derive(FromPyObject)]
pub enum CaptureTarget {
    Monitor(Monitor),
    Window(Window),
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureTargetError {
    #[error("Failed to Monitor to GraphicsCaptureItem")]
    MonitorConversionError,
    #[error("Failed to Window to GraphicsCaptureItem")]
    WindowConversionError,
}

// Make CaptureTarget convertible to GraphicsCaptureItem for all enum variants
impl TryInto<GraphicsCaptureItem> for CaptureTarget {
    type Error = CaptureTargetError;

    fn try_into(self) -> Result<GraphicsCaptureItem, Self::Error> {
        match self {
            CaptureTarget::Monitor(monitor) => monitor
                .try_into()
                .map_err(|_| CaptureTargetError::MonitorConversionError),
            CaptureTarget::Window(window) => window
                .try_into()
                .map_err(|_| CaptureTargetError::WindowConversionError),
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum ColorFormat {
    Rgba8 = 28,
}

impl Default for ColorFormat {
    fn default() -> Self {
        Self::Rgba8
    }
}
