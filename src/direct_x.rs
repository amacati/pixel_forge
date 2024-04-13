// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::Interface;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_10_1,
    D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_9_1, D3D_FEATURE_LEVEL_9_2,
    D3D_FEATURE_LEVEL_9_3,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;

#[derive(thiserror::Error, Eq, PartialEq, Clone, Debug)]
pub enum DirectXError {
    #[error("Failed to create DirectX device with the recommended feature levels")]
    FeatureLevelNotSatisfied,
    #[error("Windows API Error: {0}")]
    WindowsError(#[from] windows::core::Error),
}

impl From<DirectXError> for PyErr {
    fn from(error: DirectXError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

/// Used To Send DirectX Device Across Threads
pub struct SendDirectX<T>(pub T);

impl<T> SendDirectX<T> {
    /// Create A New `SendDirectX` Instance
    ///
    /// # Arguments
    ///
    /// * `device` - The DirectX Device
    ///
    /// # Returns
    ///
    /// Returns A New `SendDirectX` Instance
    pub const fn new(device: T) -> Self {
        Self(device)
    }
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<T> Send for SendDirectX<T> {}

/// Create `ID3D11Device` and `ID3D11DeviceContext`
pub fn create_d3d_device() -> Result<(ID3D11Device, ID3D11DeviceContext), DirectXError> {
    // Array of Direct3D feature levels.
    // The feature levels are listed in descending order of capability.
    // The highest feature level supported by the system is at index 0.
    // The lowest feature level supported by the system is at the last index.
    let feature_flags = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
        D3D_FEATURE_LEVEL_9_3,
        D3D_FEATURE_LEVEL_9_2,
        D3D_FEATURE_LEVEL_9_1,
    ];

    let mut d3d_device = None;
    let mut feature_level = D3D_FEATURE_LEVEL::default();
    let mut d3d_device_context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_flags),
            D3D11_SDK_VERSION,
            Some(&mut d3d_device),
            Some(&mut feature_level),
            Some(&mut d3d_device_context),
        )?;
    };

    if feature_level != D3D_FEATURE_LEVEL_11_1 {
        return Err(DirectXError::FeatureLevelNotSatisfied);
    }

    Ok((d3d_device.unwrap(), d3d_device_context.unwrap()))
}

/// Create `IDirect3DDevice` From `ID3D11Device`
pub fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice, DirectXError> {
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
    let device: IDirect3DDevice = inspectable.cast()?;

    Ok(device)
}
