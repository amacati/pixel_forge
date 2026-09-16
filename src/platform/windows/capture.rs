use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use numpy::{PyArray3, PyArrayMethods};
use parking_lot::{Condvar, Mutex};

use windows::core::{IInspectable, Interface, Ref};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

use super::direct_x::{create_d3d_device, create_direct3d_device};
use super::frame::Readback;
use super::monitor::Monitor;
use super::window::Window;

/// Return RGBA so we don't have to shuffle channels ourselves.
const PIXEL_FORMAT: DirectXPixelFormat = DirectXPixelFormat::R8G8B8A8UIntNormalized;

/// We need two surfaces so the compositor has one to draw into while we hold the latest one.
const BUFFERS: i32 = 2;

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("No frame available yet")]
    NoFrameAvailable,
    #[error("No frame arrived within {FIRST_FRAME_TIMEOUT:?}")]
    FirstFrameTimeout,
    #[error("Capture is not running")]
    NotRunning,
    #[error("Invalid capture target")]
    InvalidCaptureTarget,
    #[error("Failed to create a DirectX device")]
    NoDirectXDevice,
    #[error("Failed to create a staging texture")]
    NoStagingTexture,
    #[error("Frame buffer is not contiguous: {0}")]
    NotContiguous(#[from] numpy::AsSliceError),
    #[error("Windows API error: {0}")]
    WindowsError(#[from] windows::core::Error),
    #[error(transparent)]
    Monitor(#[from] super::monitor::MonitorError),
    #[error(transparent)]
    Window(#[from] super::window::WindowError),
}

impl From<CaptureError> for PyErr {
    fn from(error: CaptureError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

#[derive(FromPyObject)]
pub enum CaptureTarget {
    Monitor(Monitor),
    Window(Window),
}

impl TryFrom<CaptureTarget> for GraphicsCaptureItem {
    type Error = CaptureError;

    fn try_from(target: CaptureTarget) -> Result<Self, Self::Error> {
        Ok(match target {
            CaptureTarget::Monitor(monitor) => monitor.try_into()?,
            CaptureTarget::Window(window) => window.try_into()?,
        })
    }
}

/// Store the newest frame to prevent the compositor from checking it back into the pool while we
/// may want to read it.
#[derive(Default)]
struct Latest {
    frame: Mutex<Option<Direct3D11CaptureFrame>>,
    arrived: Condvar,
}

struct Session {
    pool: Direct3D11CaptureFramePool,
    capture: GraphicsCaptureSession,
    token: i64,
    latest: Arc<Latest>,
    readback: Mutex<Readback>,
}

/// Capture frames from a window or a monitor.
///
/// The idea is to get either a :class:`.Monitor` or a :class:`.Window` as target, create a Capture
/// object, and then start a capture that tracks the latest frame. Frames are only read back from
/// the GPU and converted to NumPy arrays when the user asks for them, to avoid unnecessary copies.
#[pyclass]
pub struct Capture {
    session: Option<Session>,
}

#[pymethods]
impl Capture {
    #[new]
    pub const fn new() -> Self {
        Self { session: None }
    }

    /// Start the capture.
    ///
    /// Frames arrive on a capture worker thread and can be read with :meth:`frame`. The first frame
    /// is not available immediately, so :meth:`start` waits for it unless ``await_first_frame`` is
    /// False.
    ///
    /// Args:
    ///     capture_target: The :class:`.Monitor` or :class:`.Window` to capture.
    ///     await_first_frame: Waits for the first frame to arrive if True.
    #[pyo3(signature = (capture_target, await_first_frame=None))]
    pub fn start(
        &mut self,
        capture_target: CaptureTarget,
        await_first_frame: Option<bool>,
    ) -> Result<(), CaptureError> {
        self.stop();
        if let CaptureTarget::Window(window) = capture_target {
            if !window.valid() {
                return Err(CaptureError::InvalidCaptureTarget);
            }
        }
        init_winrt()?;
        let item: GraphicsCaptureItem = capture_target.try_into()?;
        let (device, context) = create_d3d_device()?;
        let size = item.Size()?;
        // A free threaded pool raises FrameArrived on its own worker thread, which spares us a
        // dedicated thread running a Win32 message pump just to service a dispatcher queue.
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &create_direct3d_device(&device)?,
            PIXEL_FORMAT,
            BUFFERS,
            size,
        )?;
        let latest = Arc::new(Latest::default());
        let token = pool.FrameArrived(&TypedEventHandler::new({
            let (latest, device) = (Arc::clone(&latest), device.clone());
            let pool_size = Mutex::new(size);
            move |pool: Ref<Direct3D11CaptureFramePool>, _: Ref<IInspectable>| {
                let pool = pool.ok()?;
                let frame = pool.TryGetNextFrame()?;
                let content = frame.ContentSize()?;
                latest.frame.lock().replace(frame);
                latest.arrived.notify_all();
                // The pool's surfaces keep the size they were created with, so grow them if the
                // target outgrows them.
                let mut pool_size = pool_size.lock();
                if content.Width > pool_size.Width || content.Height > pool_size.Height {
                    pool.Recreate(
                        &create_direct3d_device(&device)?,
                        PIXEL_FORMAT,
                        BUFFERS,
                        content,
                    )?;
                    *pool_size = content;
                }
                Ok(())
            }
        }))?;
        let capture = pool.CreateCaptureSession(&item)?;
        capture.StartCapture()?;
        let readback = Mutex::new(Readback::new(device, context));
        self.session = Some(Session {
            pool,
            capture,
            token,
            latest,
            readback,
        });
        if await_first_frame.unwrap_or(true) {
            self.await_first_frame()?;
        }
        Ok(())
    }

    /// :``bool``: True if a capture is running.
    #[getter]
    pub const fn active(&self) -> bool {
        self.session.is_some()
    }

    /// Stop the capture and release the frame pool, the capture session and the last frame.
    pub fn stop(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        let _ = session.capture.Close();
        let _ = session.pool.RemoveFrameArrived(session.token);
        let _ = session.pool.Close();
        // A handler could still be active on a worker thread. It only touches `Arc<Latest>`, so
        // letting it finish is safe.
        session.latest.frame.lock().take();
    }

    /// Read the latest frame back from the GPU and return it.
    ///
    /// Returns:
    ///     The frame as a 3D NumPy array with dimensions [h w 4].
    #[pyo3(name = "frame")]
    pub fn py_frame<'py>(&self, py: Python<'py>) -> Result<Bound<'py, PyArray3<u8>>, CaptureError> {
        let session = self.session.as_ref().ok_or(CaptureError::NotRunning)?;
        // Clone the frame out of the slot so that the capture thread can keep publishing. The clone
        // keeps this surface checked out.
        let frame = session
            .latest
            .frame
            .lock()
            .clone()
            .ok_or(CaptureError::NoFrameAvailable)?;
        let size = frame.ContentSize()?;
        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
        // SAFETY: a capture frame's surface is always backed by a texture, the interface asked for
        // here, and `access` keeps it alive for the duration of the call.
        let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `texture` is the frame's backing texture and `desc` a valid out parameter.
        unsafe { texture.GetDesc(&mut desc) };
        // The surface is usually larger than the content. It can be briefly smaller right after the
        // target grew, because the pool is only recreated once a frame has reported the new size,
        // so we clamp both min and max.
        let width = (size.Width.max(0) as u32).min(desc.Width);
        let height = (size.Height.max(0) as u32).min(desc.Height);

        // SAFETY: left uninitialised because the readback below writes every byte of it.
        let array = unsafe { PyArray3::<u8>::new(py, [height as usize, width as usize, 4], false) };
        // SAFETY: the array was just created and has not been handed to Python yet, so nothing else
        // can observe its buffer while the readback fills it.
        let dst = unsafe { array.as_slice_mut()? };
        let readback = &session.readback;
        py.detach(|| readback.lock().read(&texture, width, height, dst))?;
        Ok(array)
    }
}

impl Capture {
    /// Block until the capture publishes its first frame.
    fn await_first_frame(&self) -> Result<(), CaptureError> {
        let session = self.session.as_ref().ok_or(CaptureError::NotRunning)?;
        let deadline = Instant::now() + FIRST_FRAME_TIMEOUT;
        let mut frame = session.latest.frame.lock();
        while frame.is_none() {
            if session
                .latest
                .arrived
                .wait_until(&mut frame, deadline)
                .timed_out()
            {
                return Err(CaptureError::FirstFrameTimeout);
            }
        }
        Ok(())
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Put the calling thread into a WinRT apartment, once.
///
/// WinRT types can only be activated from a thread that has initialised the runtime.
fn init_winrt() -> Result<(), CaptureError> {
    thread_local! {
        static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    }
    INITIALIZED.with(|initialized| {
        if initialized.replace(true) {
            return Ok(());
        }
        // SAFETY: runs at most once per thread, and is never paired with RoUninitialize.
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            // The thread already joined a single-threaded apartment. WinRT activation works there
            // too, so this is not an error.
            Err(e) if e.code() == RPC_E_CHANGED_MODE => Ok(()),
            result => Ok(result?),
        }
    })
}
