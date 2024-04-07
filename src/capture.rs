// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

use std::mem;
use std::sync::atomic::{self, AtomicBool};
use std::sync::{mpsc, Arc};
use std::thread::{self, sleep, JoinHandle};
use std::time::Duration;

use windows::Foundation::AsyncActionCompletedHandler;
use windows::Win32::System::Threading::{GetCurrentThreadId, GetThreadId};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DispatcherQueueOptions, RoInitialize, RoUninitialize,
    DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, RO_INIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostQuitMessage, PostThreadMessageW, TranslateMessage, MSG,
    WM_QUIT,
};

use numpy::PyArray1;
use parking_lot::Mutex;
use pyo3::prelude::*;
use rand::{thread_rng, Rng};

use crate::directx_capture::DXCapture;

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("Failed to initialize WinRT")]
    WinRTInitError,
    #[error("Failed to create dispatcher queue controller")]
    WinDQControllerError,
    #[error("Failed to create dispatcher queue")]
    FailedToCreateDispatcherQueue,
    #[error("Failed to shutdown dispatcher queue")]
    WinDQShutdownError,
    #[error("Dispatcher queue complete handler failed")]
    WinDQHandlerError,
}

#[pyclass]
pub struct Capture {
    stop_thread: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    frame: Arc<Mutex<Vec<u8>>>,
}

#[pymethods]
impl Capture {
    #[new]
    pub fn new() -> Self {
        let stop_thread = Arc::new(AtomicBool::new(false));
        let frame = Arc::new(Mutex::new(Vec::new()));

        Self {
            stop_thread,
            thread: None,
            frame,
        }
    }

    pub fn start(&mut self) {
        let stop_thread = self.stop_thread.clone();
        let frame = self.frame.clone();

        self.thread = Some(thread::spawn(move || {
            let size = 10; // Size of the array
            let mut rng = thread_rng(); // Get a random number generator

            while !stop_thread.load(atomic::Ordering::Relaxed) {
                let new_frame: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
                let mut frame_guard = frame.lock();
                *frame_guard = new_frame; // Correctly handle the mutex

                sleep(Duration::from_millis(100)); // Simulate work
            }
        }));
    }

    #[getter]
    pub fn running(&self) -> bool {
        self.thread.is_some()
    }

    pub fn stop(&mut self) {
        self.stop_thread.store(true, atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap(); // Wait for the thread to finish
        }
    }

    pub fn materialize_frame(&self, py: Python) -> Py<PyArray1<u8>> {
        PyArray1::from_vec(py, self.get_frame()).to_owned()
    }

    pub fn start2(&mut self) {
        let thread_handle = thread::spawn(move || -> Result<(), CaptureError> {
            // Initialize WinRT
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED).map_err(|_| CaptureError::WinRTInitError)?;
            };

            // Create a dispatcher queue for the current thread
            let options = DispatcherQueueOptions {
                dwSize: u32::try_from(mem::size_of::<DispatcherQueueOptions>()).unwrap(),
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            };
            let controller = unsafe {
                CreateDispatcherQueueController(options)
                    .map_err(|_| CaptureError::WinDQControllerError)?
            };

            // Get current thread ID
            let thread_id = unsafe { GetCurrentThreadId() };

            // Start capture
            /*  let result = Arc::new(Mutex::new(None));
            let callback = Arc::new(Mutex::new(
                Self::new(settings.flags).map_err(GraphicsCaptureApiError::NewHandlerError)?,
            ));
            let mut dx_capture = DXCapture::new(
                settings.item,
                callback.clone(),
                settings.cursor_capture,
                settings.draw_border,
                settings.color_format,
                thread_id,
                result.clone(),
            )?;
            dx_capture.start_capture()?;
            */
            // Send halt handle
            //let halt_handle = capture.halt_handle();
            //halt_sender.send(halt_handle).unwrap();

            // Send callback
            // callback_sender.send(callback).unwrap();

            // Message loop
            let mut message = MSG::default();
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            // Shutdown dispatcher queue
            let msg_queue_shutdown = controller
                .ShutdownQueueAsync()
                .map_err(|_| CaptureError::WinDQShutdownError)?;
            // Set completion handler for graceful shutdown. TODO: Check if this is necessary
            msg_queue_shutdown
                .SetCompleted(&AsyncActionCompletedHandler::new(
                    move |_, _| -> Result<(), windows::core::Error> {
                        unsafe { PostQuitMessage(0) };
                        Ok(())
                    },
                ))
                .map_err(|_| CaptureError::WinDQHandlerError)?;

            // Final message loop
            let mut message = MSG::default();
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            // Stop capture
            // dx_capture.stop_capture();
            // Uninitialize WinRT
            unsafe { RoUninitialize() };
            // Check handler result
            /*  if let Some(e) = result.lock().take() {
                return Err(GraphicsCaptureApiError::FrameHandlerError(e));
            } */
            panic!("This should not be reached");
            Ok(())
        });
    }
}

impl Capture {
    pub fn get_frame(&self) -> Vec<u8> {
        self.frame.lock().clone()
    }
}
