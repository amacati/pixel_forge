import numpy as np

from pixel_forge import Capture


def test_create_capture():
    c = Capture()
    assert c is not None


def test_capture_thread():
    c = Capture()
    c.start()
    c.stop()


def test_capture_frame():
    c = Capture()
    c.start()
    frame = c.materialize_frame()
    c.stop()
    assert isinstance(frame, np.ndarray)


def test_capture_running():
    c = Capture()
    assert not c.running, "Capture should not be running"
    c.start()
    assert c.running, "Capture should be running"
    c.stop()
    assert not c.running, "Capture should not be running"
