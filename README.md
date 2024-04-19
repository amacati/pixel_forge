# pixel forge

<p align="center">
  <img width="256" height="256" src="https://raw.githubusercontent.com/amacati/pixel_forge/master/docs/img/pixel_forge_banner.png">
</p>

Ultra-fast screen capture in Rust with Python bindings

# Installing

The package can be installed via

```bash
pip install pixel_forge
```

# Example

```python
import time

import numpy as np
from pixel_forge import Capture, Monitor, Window, foreground_window

c = Capture()
m = Monitor()
c.start(m)
img = c.frame()
c.stop()
assert isinstance(img, np.ndarray)

w: Window = foreground_window()
c.start(w)
img1 = c.frame()
time.sleep(1)
img2 = c.frame()  # Returns the most recent frame
c.stop()
```