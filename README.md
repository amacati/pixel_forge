# pixel forge

<p align="center">
  <img width="256" height="256" src="docs/img/pixel_forge_banner.png">
</p>

Ultra-fast screen capture in Rust with Python bindings

# Example

```python
import numpy as np
from pixel_forge import Monitor, Window, foreground_window, Capture

c = Capture()
m = Monitor()
c.start(m)
img = c.frame()
c.close()
assert isinstance(img, np.ndarray)

w = foreground_window()
c.start(w)
img = c.frame()
c.close()
print(img.shape)
```