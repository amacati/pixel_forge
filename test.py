import time

from pixel_forge import Capture, enumerate_windows, foreground_window, primary_monitor

print(enumerate_windows())
print(enumerate_windows()[0].valid)
window = enumerate_windows()[0]
print(window.title)
print(foreground_window().title)
print(type(enumerate_windows()))
monitor = primary_monitor()
print(monitor.width)
print(monitor.device_name)
print(monitor.device_string)


c = Capture()
x = c.materialize_frame()
print(x, x.shape, type(x))
