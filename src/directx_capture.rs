// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::sync::Arc;

use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D};

use crate::direct_x::{create_d3d_device, create_direct3d_device, DirectXError};

#[derive(thiserror::Error, Eq, PartialEq, Clone, Debug)]
pub enum DirectXCaptureError {
    #[error("Graphics capture API is not supported")]
    Unsupported,
    #[error("Graphics capture API toggling cursor capture is not supported")]
    CursorConfigUnsupported,
    #[error("Graphics capture API toggling border capture is not supported")]
    BorderConfigUnsupported,
    #[error("Already started")]
    AlreadyStarted,
    #[error("DirectX error: {0}")]
    DirectXError(#[from] DirectXError),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

pub struct DirectXCapture {
    gc_item: GraphicsCaptureItem,
    d3d_device: ID3D11Device,
    direct3d_device: IDirect3DDevice,
    d3d_device_context: ID3D11DeviceContext,
    frame_pool: Option<Arc<Direct3D11CaptureFramePool>>,
    session: Option<GraphicsCaptureSession>,
    capture_closed_event: EventRegistrationToken,
    frame_arrived_event: EventRegistrationToken,
}

impl DirectXCapture {
    pub fn new() -> Result<Self, DirectXCaptureError> {}
}
