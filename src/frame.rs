// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::slice;

use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CPU_ACCESS_WRITE, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ_WRITE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};

use windows_result::Error as WindowsError;

use crate::capture_utils::ColorFormat;

#[derive(thiserror::Error, Debug)]
pub enum FrameError {
    #[error("Conversion to vector failed.")]
    FrameConversionFailed,
    #[error("Windows error during frame conversion")]
    FrameConversionWindowsError(#[from] WindowsError),
}

#[derive(Clone, Debug)]
pub struct Frame {
    // Texture properties
    frame_texture: ID3D11Texture2D,
    height: u32,
    width: u32,
    // Conversion devices
    d3d_device: ID3D11Device,
    context: ID3D11DeviceContext,
}

impl Frame {
    pub fn new(
        frame_texture: ID3D11Texture2D,
        height: u32,
        width: u32,
        d3d_device: ID3D11Device,
        context: ID3D11DeviceContext,
    ) -> Self {
        Self {
            frame_texture,
            height,
            width,
            d3d_device,
            context,
        }
    }

    pub fn materialize(&self) -> Result<Vec<u8>, FrameError> {
        // Create a texture that CPU can read
        let texture_desc = D3D11_TEXTURE2D_DESC {
            Width: self.width,
            Height: self.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT(ColorFormat::default() as i32),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32 | D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: 0,
        };

        let mut texture = None;
        unsafe {
            self.d3d_device
                .CreateTexture2D(&texture_desc, None, Some(&mut texture))?;
        };
        let texture = texture.unwrap();

        // Copy the real texture to copy texture
        unsafe {
            self.context.CopyResource(&texture, &self.frame_texture);
        };

        // Map the texture to enable CPU access
        let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context.Map(
                &texture,
                0,
                D3D11_MAP_READ_WRITE,
                0,
                Some(&mut mapped_resource),
            )?;
        };

        // Get the mapped resource data slice
        let mapped_frame_data: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(
                mapped_resource.pData.cast(),
                (self.height * mapped_resource.RowPitch) as usize,
            )
        };
        let frame_vec = mapped_frame_data.to_vec();
        Ok(frame_vec)
    }
}

impl TryFrom<Frame> for Vec<u8> {
    type Error = FrameError;

    fn try_from(frame: Frame) -> Result<Vec<u8>, Self::Error> {
        Ok(frame.materialize()?)
    }
}

impl TryFrom<&Frame> for Vec<u8> {
    type Error = FrameError;

    fn try_from(frame: &Frame) -> Result<Vec<u8>, Self::Error> {
        Ok(frame.materialize()?)
    }
}
