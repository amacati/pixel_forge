import numpy as np
import pytest

from pixel_forge import Capture, Monitor, primary_monitor


@pytest.fixture(scope="session")
def monitor():
    yield primary_monitor()


def test_create_capture():
    c = Capture()
    assert c is not None


def test_capture_thread(monitor: Monitor):
    c = Capture()
    c.start(monitor, await_first_frame=True)
    c.stop()


def test_capture_frame(monitor: Monitor):
    c = Capture()
    c.start(monitor, await_first_frame=True)
    frame = c.frame()
    c.stop()
    assert isinstance(frame, np.ndarray)


def test_capture_running(monitor: Monitor):
    c = Capture()
    assert not c.running, "Capture should not be running"
    c.start(monitor, await_first_frame=True)
    assert c.running, "Capture should be running"
    c.stop()
    assert not c.running, "Capture should not be running"


def test_capture_valid_frame(monitor: Monitor):
    c = Capture()
    assert not c.running, "Capture should not be running"
    c.start(monitor, await_first_frame=True)
    frame = c.frame()
    assert isinstance(frame, np.ndarray), "Frame should be a numpy array"
    assert frame.shape[0] > 0, "Frame must not be empty"
    c.stop()
    assert not c.running, "Capture should not be running"
