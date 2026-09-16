use std::slice;

use windows::Win32::Graphics::Direct3D11::{
    D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

use super::capture::CaptureError;

/// Read capture frames back from the GPU into CPU memory.
///
/// The staging texture is allocated once and reused for every frame. D3D11 defers resource
/// destruction, and freed textures otherwise pile up in the graphics kernel's paged pool. It is
/// only rebuilt when the capture target resizes.
pub struct Readback {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    staging: Option<Staging>,
}

struct Staging {
    texture: ID3D11Texture2D,
    width: u32,
    height: u32,
}

impl Readback {
    pub const fn new(device: ID3D11Device, context: ID3D11DeviceContext) -> Self {
        Self {
            device,
            context,
            staging: None,
        }
    }

    /// Copy the top-left [`width`, `height`] region of `source` into `dst` as RGBA rows
    pub fn read(
        &mut self,
        source: &ID3D11Texture2D,
        width: u32,
        height: u32,
        dst: &mut [u8],
    ) -> Result<(), CaptureError> {
        let staging = self.staging(width, height)?;
        let region = D3D11_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: width,
            bottom: height,
            back: 1,
        };
        // SAFETY: `staging` and `source` are both live textures of the same format, and `region`
        // lies within `source` because the frame pool surface is never smaller than the content.
        // Subresource 0 is the only subresource of either texture.
        unsafe {
            self.context
                .CopySubresourceRegion(&staging, 0, 0, 0, 0, source, 0, Some(&region))
        };
        Mapped::new(&self.context, &staging)?.unpack(dst, width as usize, height as usize);
        Ok(())
    }

    /// The cached staging texture, rebuilt if the target resized since the last frame.
    fn staging(&mut self, width: u32, height: u32) -> Result<ID3D11Texture2D, CaptureError> {
        if let Some(s) = &self.staging
            && s.width == width
            && s.height == height
        {
            return Ok(s.texture.clone());
        }
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            // Read access only. A CPU-writable mapping can land in write-combined memory, which
            // is super slow to read back from.
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            ..Default::default()
        };
        let mut texture = None;
        // SAFETY: `desc` is a fully initialised staging description and `texture` is a valid
        // out parameter.
        unsafe {
            self.device
                .CreateTexture2D(&desc, None, Some(&mut texture))?
        };
        let texture = texture.ok_or(CaptureError::NoStagingTexture)?;
        self.staging = Some(Staging {
            texture: texture.clone(),
            width,
            height,
        });
        Ok(texture)
    }
}

/// A mapped staging texture.
///
/// D3D11 denies the GPU access to a resource between `Map` and `Unmap`, so we ensure Unmap is
/// called by putting it into `Drop`.
struct Mapped<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
    resource: D3D11_MAPPED_SUBRESOURCE,
}

impl<'a> Mapped<'a> {
    fn new(
        context: &'a ID3D11DeviceContext,
        texture: &'a ID3D11Texture2D,
    ) -> Result<Self, CaptureError> {
        let mut resource = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: `texture` is a staging texture created with D3D11_CPU_ACCESS_READ, so a read map
        // is permitted. Subresource 0 is its only subresource, and `resource` is a valid out
        // parameter. The matching Unmap happens in `Drop`.
        unsafe { context.Map(texture, 0, D3D11_MAP_READ, 0, Some(&mut resource))? };
        Ok(Self {
            context,
            texture,
            resource,
        })
    }

    /// Copy `height` rows of `width` pixels into `dst`, dropping the driver's row padding.
    fn unpack(&self, dst: &mut [u8], width: usize, height: usize) {
        let pitch = self.resource.RowPitch as usize;
        let row = width * 4;
        // SAFETY: the mapping covers `height` rows of `pitch` bytes and stays valid until this
        // guard drops, which happens after the copy below.
        let src =
            unsafe { slice::from_raw_parts(self.resource.pData.cast::<u8>(), pitch * height) };
        if pitch == row {
            dst.copy_from_slice(src); // No padding, so the rows are already contiguous
            return;
        }
        for (dst, src) in dst.chunks_exact_mut(row).zip(src.chunks_exact(pitch)) {
            dst.copy_from_slice(&src[..row]);
        }
    }
}

impl Drop for Mapped<'_> {
    fn drop(&mut self) {
        // SAFETY: `new` mapped this exact texture and subresource, and a guard is only built on a
        // successful map, so this is the single matching unmap.
        unsafe { self.context.Unmap(self.texture, 0) };
    }
}
