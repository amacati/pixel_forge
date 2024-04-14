// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::mem;
use std::sync::atomic::{self, AtomicBool, AtomicI64};
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

use ctrlc;
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
    thread: Option<JoinHandle<Result<(), CaptureError>>>,
    frame: Arc<Mutex<Option<Frame>>>,
    frame_cnt: Arc<AtomicI64>,
}

impl Capture {
    pub fn materialize_frame(&self) -> Result<Vec<u8>, CaptureError> {
        // Keep the lock_guard in scope to ensure the lock is still valid when we convert it to a
        // Vec<u8>
        let lock_guard = self.frame.lock();
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

        let stop_flag = stop_thread.clone();
        ctrlc::set_handler(move || stop_flag.store(true, atomic::Ordering::Relaxed))
            .expect("Error setting Ctrl-C handler.");

        Self {
            stop_thread,
            capture_close,
            thread: None,
            frame: Arc::new(Mutex::new(None)),
            frame_cnt: Arc::new(AtomicI64::new(0)),
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

        // Clone Arc capture struct members to use them in thread without borrowing
        let stop_thread = self.stop_thread.clone();
        let capture_close = self.capture_close.clone();
        let frame = self.frame.clone();
        let frame_cnt = self.frame_cnt.clone();

        // Create a thread to run the capture
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

            // Start capture here
            // Create DirectX devices
            let (d3d_device, d3d_device_context) = create_d3d_device()?;
            let direct3d_device = create_direct3d_device(&d3d_device)?;
            // Create frame pool
            let pixel_format = DirectXPixelFormat(ColorFormat::default() as i32);
            let frame_pool = Direct3D11CaptureFramePool::Create(
                &direct3d_device,
                pixel_format,
                1,
                gc_item.Size()?,
            )?;
            let frame_pool = Arc::new(frame_pool);
            // Create capture session
            let session = frame_pool.CreateCaptureSession(&gc_item)?;

            // Set capture session closed event
            // We need to create a clone of stop_thread to use it in the closure as we still need to
            // use the original stop_thread in the frame_arrived event
            let _stop_thread = stop_thread.clone();
            let capture_closed_event_token = gc_item.Closed(&TypedEventHandler::<
                GraphicsCaptureItem,
                IInspectable,
            >::new({
                move |_, _| {
                    _stop_thread.store(true, atomic::Ordering::Relaxed);
                    capture_close.store(true, atomic::Ordering::Relaxed);
                    // Stop the message loop
                    unsafe {
                        PostThreadMessageW(
                            thread_id,
                            WM_QUIT,
                            WPARAM::default(),
                            LPARAM::default(),
                        )?;
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
                let d3d_device = d3d_device.clone();
                let context = d3d_device_context.clone();
                let capture_frame = frame.clone();
                // Clone stop_thread flag again to use it in another closure
                let _stop_thread = stop_thread.clone();

                let mut last_size = gc_item.Size()?;
                let direct3d_device_recreate = SendDirectX::new(direct3d_device.clone());

                move |frame, _| {
                    // Return immediately if the thread is stopped
                    if _stop_thread.load(atomic::Ordering::Relaxed) {
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
                    *capture_frame.lock() = Some(Frame::new());
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
                    frame_cnt.fetch_add(1, atomic::Ordering::Relaxed);
                }
            }
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
            gc_item
                .RemoveClosed(capture_closed_event_token)
                .expect("Failed to remove Capture Session Closed event handler");

            // Uninitialize WinRT
            unsafe { RoUninitialize() };
            Ok(())
        });
        self.thread = Some(capture_thread);

        // Wait for the first frame to be ready. Also checks if stop_thread is set to true by the
        // Ctrl-C handler to enable interrupts during the first frame wait
        if await_first_frame.unwrap_or(false) {
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
            thread
                .join()
                .unwrap()
                .expect("Failed to join thread on capture stop"); // Wait for the thread to finish
        }
        self.frame.lock().take(); // Clear the frame when the capture is stopped
    }

    // Convert the frame into a numpy array and return it to the user
    #[pyo3(name = "frame")]
    pub fn py_frame(&self, py: Python) -> PyResult<Py<PyArray1<u8>>> {
        if self.thread.is_none() {
            return Err(PyRuntimeError::new_err("Capture thread is not running."));
        }
        Ok(PyArray1::from_vec(py, self.materialize_frame()?).to_owned())
    }

    pub fn frame_cnt(&self) -> i64 {
        self.frame_cnt.load(atomic::Ordering::Relaxed)
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
