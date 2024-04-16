// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::mem;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::thread::{self, sleep, JoinHandle};
use std::time::Duration;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use windows::core::{IInspectable, Interface};
use windows::Foundation::AsyncActionCompletedHandler;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DispatcherQueueOptions, RoInitialize, RoUninitialize,
    DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, RO_INIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
};
use windows_result::Error as WindowsError;

use numpy::ndarray::{self, s};
use numpy::PyArray3;
use numpy::ToPyArray;
use parking_lot::Mutex;

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
    #[error("Capture thread exited unexpectedly with an error.")]
    CaptureThreadError,
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
    stopped_thread: Arc<AtomicBool>, // Flag to check if the capture thread is still responsive when stopping
    thread: Option<JoinHandle<Result<(), CaptureError>>>,
    frame: Arc<Mutex<Option<Frame>>>,
}

#[pymethods]
impl Capture {
    #[new]
    pub fn new() -> Self {
        Self {
            stop_thread: Arc::new(AtomicBool::new(false)),
            stopped_thread: Arc::new(AtomicBool::new(false)),
            thread: None,
            frame: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(
        &mut self,
        capture_target: CaptureTarget,
        await_first_frame: Option<bool>,
    ) -> Result<(), CaptureError> {
        let gc_item: GraphicsCaptureItem = capture_target
            .try_into()
            .expect("Failed to convert CaptureTarget to GraphicsCaptureItem");

        self.stop_thread.store(false, atomic::Ordering::Relaxed);
        self.stopped_thread.store(false, atomic::Ordering::Relaxed);
        // Clone Arc capture struct members to use them in thread without borrowing
        let stop_thread = self.stop_thread.clone();
        let stopped_thread = self.stopped_thread.clone();
        let frame = self.frame.clone();

        // Create a thread to run the capture
        let capture_thread = thread::spawn(move || -> Result<(), CaptureError> {
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED)?; // Initialize the Windows Runtime
            };
            // Create a dispatcher queue for the current thread
            let options = DispatcherQueueOptions {
                dwSize: u32::try_from(mem::size_of::<DispatcherQueueOptions>()).unwrap(),
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            };
            let controller = unsafe { CreateDispatcherQueueController(options)? };

            // Create DirectX devices
            let (d3d_device, d3d_device_context) = create_d3d_device()?;
            let direct3d_device = create_direct3d_device(&d3d_device)?;
            // Create frame pool and an associated capture session
            let pixel_format = DirectXPixelFormat(ColorFormat::default() as i32);
            let frame_pool = Arc::new(Direct3D11CaptureFramePool::Create(
                &direct3d_device,
                pixel_format,
                1,
                gc_item.Size()?,
            )?);
            let session = frame_pool.CreateCaptureSession(&gc_item)?;

            // Set frame pool frame arrived event
            let frame_arrived_event_token = frame_pool.FrameArrived(&TypedEventHandler::<
                Direct3D11CaptureFramePool,
                IInspectable,
            >::new({
                // Init
                let frame_pool = frame_pool.clone();
                let d3d_device = d3d_device.clone();
                let context = d3d_device_context.clone();
                let capture_frame = frame.clone();
                // Clone stop_thread flag again to use it in another closure
                let stop_thread = stop_thread.clone();

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
                    let frame_content_size = frame.ContentSize()?;
                    let frame_surface = frame.Surface()?;
                    // Convert surface to texture
                    let frame_dxgi_interface =
                        frame_surface.cast::<IDirect3DDxgiInterfaceAccess>()?;
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
                    // Create a frame
                    *capture_frame.lock() = Some(Frame::new(
                        frame_texture,
                        texture_height,
                        texture_width,
                        d3d_device.clone(),
                        context.clone(),
                    ));
                    Result::Ok(())
                }
            }))?;
            session.StartCapture()?;

            // Create message loops. Pump messages while the message is not WM_QUIT and the
            // stop_thread flag is not set
            let mut msg = MSG::default();
            unsafe {
                while GetMessageW(&mut msg, None, 0, 0).as_bool()
                    & !stop_thread.load(atomic::Ordering::Relaxed)
                {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            // Set stopped_thread flag to true to signal that the thread has stopped its main
            // message loop. If the window is closed while the capture is running, the thread will
            // silently exit and calling join on it will hang indefinitely. Therefore we test this
            // flag in the in the stop method to make sure that the thread is still responsive
            // before joining it
            stopped_thread.store(true, atomic::Ordering::Relaxed);
            // Shutdown dispatcher queue
            let async_shutdown = controller.ShutdownQueueAsync()?;
            async_shutdown.SetCompleted(&AsyncActionCompletedHandler::new(
                move |_, _| -> Result<(), windows::core::Error> {
                    unsafe { PostQuitMessage(0) };
                    Ok(())
                },
            ))?;

            // Stop capture
            frame_pool
                .RemoveFrameArrived(frame_arrived_event_token)
                .expect("Failed to remove Frame Arrived event handler");
            frame_pool.Close().expect("Failed to Close Frame Pool");
            session.Close().expect("Failed to Close Capture Session");
            unsafe { RoUninitialize() };
            Ok(())
        });
        self.thread = Some(capture_thread);

        // Wait for the first frame to be ready. Also checks if stop_thread is set to true by the
        if await_first_frame.unwrap_or(true) {
            while self.frame.lock().is_none() & !self.stop_thread.load(atomic::Ordering::Relaxed) {
                sleep(Duration::from_millis(10));
            }
        }
        Ok(())
    }

    // Python property to check if the capture thread is running
    #[getter]
    pub fn active(&self) -> bool {
        self.thread.is_some()
    }

    // Stop the capture thread and wait for it to join
    pub fn stop(&mut self) {
        self.stop_thread.store(true, atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let tstart = std::time::Instant::now();
            // Wait for the thread to respond to the stop signal for a maximum of 100ms
            while !self.stopped_thread.load(atomic::Ordering::Relaxed)
                & (std::time::Instant::now().duration_since(tstart) < Duration::from_millis(100))
            {
                sleep(Duration::from_millis(10));
            }
            if self.stopped_thread.load(atomic::Ordering::Relaxed) {
                let _ = thread.join();
            }
        }
        self.frame.lock().take(); // Clear the frame when the capture is stopped
    }

    // Convert the frame into a numpy array and return it to the user
    #[pyo3(name = "frame")]
    pub fn py_frame(&self, py: Python) -> PyResult<Py<PyArray3<u8>>> {
        if self.thread.is_none() {
            return Err(PyRuntimeError::new_err("Capture thread is not running."));
        }
        let frame_guard = self.frame.lock();
        let frame = frame_guard.as_ref().ok_or(CaptureError::NoFrameAvailable)?;
        let data = frame.materialize()?;
        let img_array = ndarray::arr1(data);
        // For some reason, only the height of the frame is correct and the texture includes a white
        // border. We calculate the width according to the number of available elements and later
        // crop the frame back to the intended size
        let height: usize = frame.height.try_into()?;
        let dims: [usize; 3] = [height, data.len() / height / 4, 4];
        let img_array = img_array
            .into_shape(dims)
            .expect("Failed to reshape frame into the correct dimensions");
        let width: usize = frame.width.try_into()?;
        // Crop image into the correct dimensions and discard any borders
        let img_array = img_array.slice(s![0..height, 0..width, ..]).to_pyarray(py);
        Ok(img_array.to_owned())
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
