// This code has been adapted from https://github.com/NiiightmareXD/windows-capture
use std::{
    sync::atomic::{self, AtomicBool},
    thread::{self, JoinHandle},
};

use numpy::PyArray1;

use pyo3::prelude::*;

use parking_lot::Mutex;
use std::sync::Arc;

use std::thread::sleep;
use std::time::Duration;

use rand::{thread_rng, Rng};

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

    pub fn start_capture_thread(&mut self) {
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

    pub fn stop_capture_thread(&mut self) {
        self.stop_thread.store(true, atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap(); // Wait for the thread to finish
        }
    }

    pub fn materialize_frame(&self, py: Python) -> Py<PyArray1<u8>> {
        PyArray1::from_vec(py, self.get_frame()).to_owned()
    }
}

impl Capture {
    pub fn get_frame(&self) -> Vec<u8> {
        self.frame.lock().clone()
    }
}
