import matplotlib.pyplot as plt

from pixel_forge import Capture, Window

c = Capture()
window = Window("Untitled - Notepad")
c.start(window, await_first_frame=True)
x = c.frame()
c.stop()
d = len(x) // 1440 // 4
x = x.reshape((d, 1440, 4))
plt.imshow(x)
plt.show()
