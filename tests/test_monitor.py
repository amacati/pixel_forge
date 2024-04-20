from pixel_forge import Monitor, enumerate_monitors, primary_monitor


def test_monitor_init():
    m = Monitor()  # primary monitor
    assert isinstance(m, Monitor)
    m = Monitor(1)  # Also primary monitor, but with explicit id
    assert isinstance(m, Monitor)


def test_primary_monitor():
    monitor = primary_monitor()
    assert isinstance(monitor, Monitor)
    assert monitor.index == 1


def test_enumerate_monitors():
    monitors = enumerate_monitors()
    assert len(monitors) > 0
    for monitor in monitors:
        assert isinstance(monitor, Monitor)


def test_monitor_properties():
    monitor = Monitor()
    assert monitor.width > 0
    assert monitor.height > 0
    assert monitor.index >= 0
    assert monitor.refresh_rate > 0
    assert isinstance(monitor.device_name, str)
    assert isinstance(monitor.device_string, str)
