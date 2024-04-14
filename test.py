from pixel_forge import Capture, Window
import time

c = Capture()
window = Window("Untitled - Notepad")
c.start(window, await_first_frame=True)
t = time.perf_counter()
time.sleep(10)
x = c.frame()
print(x, x.shape, type(x))

cnt = c.frame_cnt()
dt = time.perf_counter() - t

print(f"Frame count: {cnt}, Time elapsed: {dt:.2f}s, FPS: {cnt / dt:.2f}fps")
