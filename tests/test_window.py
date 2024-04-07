from pixel_forge import Window, enumerate_windows


def test_enumerate_windows():
    windows = enumerate_windows()
    assert len(windows) > 0
    for window in windows:
        assert isinstance(window, Window)


def test_window_properties():
    window = enumerate_windows()[0]
    assert window.valid
    assert isinstance(window.title, str)
