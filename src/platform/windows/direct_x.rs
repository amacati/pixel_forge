use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL,
    D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
    ID3D11DeviceContext, ID3D11Multithread,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::core::Interface;

use super::capture::CaptureError;

/// Create the D3D11 device and its immediate context.
///
/// Falls back to the WARP software rasterizer when there is no capable GPU over RDP.
pub fn create_d3d_device() -> Result<(ID3D11Device, ID3D11DeviceContext), CaptureError> {
    // Descending order of capability. 11_0 is our minimum required version
    let levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
    let device = create_device(D3D_DRIVER_TYPE_HARDWARE, &levels)
        .or_else(|_| create_device(D3D_DRIVER_TYPE_WARP, &levels))?;

    // The Windows Graphics Capture runtime touches the device from its own worker threads while we
    // read frames back on the caller's thread. The immediate context is not free-threaded, so ask
    // D3D11 to serialize it for us.
    let multithread: ID3D11Multithread = device.1.cast()?;
    // SAFETY: `multithread` was just queried from the immediate context, so the call is valid.
    let _ = unsafe { multithread.SetMultithreadProtected(true) };
    Ok(device)
}

fn create_device(
    driver: D3D_DRIVER_TYPE,
    levels: &[D3D_FEATURE_LEVEL],
) -> Result<(ID3D11Device, ID3D11DeviceContext), CaptureError> {
    let (mut device, mut context) = (None, None);
    // SAFETY: `levels` outlives the call, and both out parameters are valid. A null adapter with an
    // explicit driver type lets D3D11 pick the adapter itself.
    unsafe {
        D3D11CreateDevice(
            None,
            driver,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
    }
    match (device, context) {
        (Some(device), Some(context)) => Ok((device, context)),
        _ => Err(CaptureError::NoDirectXDevice),
    }
}

/// Wrap an [`ID3D11Device`] as the WinRT device that the capture frame pool expects.
pub fn create_direct3d_device(device: &ID3D11Device) -> windows::core::Result<IDirect3DDevice> {
    let dxgi: IDXGIDevice = device.cast()?;
    // SAFETY: `dxgi` is a live DXGI device queried from `device`.
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
    inspectable.cast()
}
