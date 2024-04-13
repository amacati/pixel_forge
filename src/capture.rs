// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::mem;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::thread::{self, sleep, JoinHandle};
use std::time::Duration;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::{Error, IInspectable, Interface};
use windows::Foundation::AsyncActionCompletedHandler;
use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::{IDirect3DDevice, IDirect3DSurface};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_TEXTURE2D_DESC,
};
use windows::Win32::System::Threading::{GetCurrentThreadId, GetThreadId};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DispatcherQueueOptions, RoInitialize, RoUninitialize,
    DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, RO_INIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, PostThreadMessageW, TranslateMessage, MSG,
    WM_QUIT,
};
use windows_result::Error as WindowsError;

use numpy::PyArray1;
use parking_lot::Mutex;

use rand::{thread_rng, Rng};

use crate::capture_utils::{CaptureTarget, ColorFormat};
use crate::direct_x::{create_d3d_device, create_direct3d_device, DirectXError, SendDirectX};
use crate::frame::{Frame, FrameError};

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("No frame available yet.")]
    NoFrameAvailable,
    #[error("Windows error during Capture.")]
    WindowsError(#[from] WindowsError),
    #[error("DirectX error during Capture.")]
    DirectXError(#[from] DirectXError),
    #[error("Frame could not be materialized.")]
    FrameConversionError(#[from] FrameError),
}

impl From<CaptureError> for PyErr {
    fn from(error: CaptureError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
}

// The Capture struct is the central struct of pixel_forge. The main idea is to get either a monitor
// or a window as target, create a Capture struct, and then start a capture thread that will update
// the texture of the Capture struct whenever a new frame is available. We only materialize the
// frame when the user requests it to avoid unnecessary copies.
#[pyclass]
pub struct Capture {
    stop_thread: Arc<AtomicBool>,
    capture_close: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    frame: Arc<Mutex<Option<Vec<u8>>>>,
    frame2: Arc<Mutex<Option<Frame>>>,
}

impl Capture {
    pub fn materialize_frame(&self) -> Result<Vec<u8>, CaptureError> {
        self.frame
            .lock()
            .clone()
            .ok_or(CaptureError::NoFrameAvailable)
    }

    pub fn materialize_frame2(&self) -> Result<Vec<u8>, CaptureError> {
        let lock_guard = self.frame2.lock();
        let frame_ref = lock_guard.as_ref().ok_or(CaptureError::NoFrameAvailable)?;
        Ok(Vec::<u8>::try_from(frame_ref)?)
    }
}

#[pymethods]
impl Capture {
    #[new]
    pub fn new() -> Self {
        let stop_thread = Arc::new(AtomicBool::new(false));
        let capture_close = Arc::new(AtomicBool::new(false));
        let frame = Arc::new(Mutex::new(None));

        Self {
            stop_thread,
            capture_close,
            thread: None,
            frame,
            frame2: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&mut self, capture_target: CaptureTarget, await_first_frame: Option<bool>) {
        let gc_item: GraphicsCaptureItem = capture_target
            .try_into()
            .expect("Failed to convert CaptureTarget to GraphicsCaptureItem");
        let stop_thread = self.stop_thread.clone();
        let frame = self.frame.clone();

        self.thread = Some(thread::spawn(move || {
            let size = 10; // Size of the array
            let mut rng = thread_rng(); // Get a random number generator

            while !stop_thread.load(atomic::Ordering::Relaxed) {
                sleep(Duration::from_millis(100)); // Simulate work
                (*frame.lock()) = Some((0..size).map(|_| rng.gen()).collect());
            }
        }));

        if await_first_frame.unwrap_or(false) {
            while self.frame.lock().is_none() {
                sleep(Duration::from_millis(10));
            }
        }
    }

    pub fn start2(
        &mut self,
        capture_target: CaptureTarget,
        await_first_frame: Option<bool>,
    ) -> Result<(), CaptureError> {
        let gc_item: GraphicsCaptureItem = capture_target
            .try_into()
            .expect("Failed to convert CaptureTarget to GraphicsCaptureItem");

        let capture_thread = thread::spawn(move || -> Result<(), CaptureError> {
            // Initialize Windows Runtime
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED)?;
            };
            // Create a dispatcher queue for the current thread
            let options = DispatcherQueueOptions {
                dwSize: u32::try_from(mem::size_of::<DispatcherQueueOptions>()).unwrap(),
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            };
            let controller = unsafe { CreateDispatcherQueueController(options)? };
            let thread_id = unsafe { GetCurrentThreadId() };

            Ok(())
        });

        let stop_thread = self.stop_thread.clone();
        let capture_close = self.capture_close.clone();
        let frame = self.frame.clone();

        // Create DirectX devices
        let (d3d_device, d3d_device_context) = create_d3d_device()?;
        let direct3d_device = create_direct3d_device(&d3d_device)?;

        // Create frame pool
        let pixel_format = DirectXPixelFormat(ColorFormat::default() as i32);
        let frame_pool =
            Direct3D11CaptureFramePool::Create(&direct3d_device, pixel_format, 1, gc_item.Size()?)?;
        let frame_pool = Arc::new(frame_pool);
        // Create capture session
        let session = frame_pool.CreateCaptureSession(&gc_item)?;
        let mut buffer = vec![0u8; 3840 * 2160 * 4];

        let thread_id = unsafe { GetCurrentThreadId() };

        // Set capture session closed event
        let capture_closed_event_token = gc_item.Closed(&TypedEventHandler::<
            GraphicsCaptureItem,
            IInspectable,
        >::new({
            move |_, _| {
                stop_thread.store(true, atomic::Ordering::Relaxed);
                capture_close.store(true, atomic::Ordering::Relaxed);
                // Stop the message loop
                unsafe {
                    PostThreadMessageW(thread_id, WM_QUIT, WPARAM::default(), LPARAM::default())?;
                };
                Result::Ok(())
            }
        }))?;

        // Set frame pool frame arrived event
        let frame_arrived_event_token = frame_pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new({
            // Init
            let frame_pool = frame_pool.clone();
            let stop_thread = self.stop_thread.clone();
            let d3d_device = d3d_device.clone();
            let context = d3d_device_context.clone();
            let capture_frame = self.frame2.clone();

            let mut last_size = gc_item.Size()?;
            let direct3d_device_recreate = SendDirectX::new(direct3d_device.clone());

            move |frame, _| {
                // Return immediately if the thread is stopped
                if stop_thread.load(atomic::Ordering::Relaxed) {
                    return Ok(());
                }
                // Get frame
                let frame = frame
                    .as_ref()
                    .expect("FrameArrived parameter unexpectedly returned None.")
                    .TryGetNextFrame()?;
                // Get frame time, content size and surface
                let timespan = frame.SystemRelativeTime()?;
                let frame_content_size = frame.ContentSize()?;
                let frame_surface = frame.Surface()?;
                // Convert surface to texture
                let frame_dxgi_interface = frame_surface.cast::<IDirect3DDxgiInterfaceAccess>()?;
                let frame_texture =
                    unsafe { frame_dxgi_interface.GetInterface::<ID3D11Texture2D>()? };

                // Get texture settings
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                unsafe { frame_texture.GetDesc(&mut desc) }

                // Check if the size has been changed, and recreate the frame pool if necessary
                if frame_content_size.Width != last_size.Width
                    || frame_content_size.Height != last_size.Height
                {
                    let direct3d_device_recreate = &direct3d_device_recreate;
                    frame_pool.Recreate(
                        &direct3d_device_recreate.0,
                        pixel_format,
                        1,
                        frame_content_size,
                    )?;
                    last_size = frame_content_size;
                    return Ok(());
                }
                // Set width & height
                let texture_width = desc.Width;
                let texture_height = desc.Height;

                println!("Frame arrived: {}x{}", texture_width, texture_height);
                // Create a frame
                let frame = Frame::new();
                (*capture_frame.lock()) = Some(frame);
                Result::Ok(())
            }
        }))?;
        session.StartCapture()?;

        // Wait for the first frame to be ready if requested
        if await_first_frame.unwrap_or(false) {
            while self.frame2.lock().is_none() {
                sleep(Duration::from_millis(10));
            }
        }

        Ok(())
    }

    // Python property to check if the capture thread is running
    #[getter]
    pub fn running(&self) -> bool {
        self.thread.is_some()
    }

    // Stop the capture thread and wait for it to join
    pub fn stop(&mut self) {
        self.stop_thread.store(true, atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap(); // Wait for the thread to finish
        }
    }

    // Convert the frame into a numpy array and return it to the user
    #[pyo3(name = "frame")]
    pub fn py_frame(&self, py: Python) -> PyResult<Py<PyArray1<u8>>> {
        Ok(PyArray1::from_vec(py, self.materialize_frame()?).to_owned())
    }
}

// Drop trait implementation to stop the capture thread when the Capture struct is dropped. This
// trait is also executed when the Capture struct goes out of scope in Python, making sure that the
// capture thread is stopped
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}
