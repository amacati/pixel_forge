# pixel forge
[![Python Version]][Python Version URL] [![Rust]][Rust URL] [![Version]][Version URL] [![Licence]][License URL] [![CI]][CI URL] [![Docs Status]][Docs URL]


[Licence]: https://img.shields.io/badge/license-MIT-blue
[License URL]: https://github.com/amacati/pixel_forge/blob/master/LICENSE

[CI]: https://img.shields.io/github/actions/workflow/status/amacati/pixel_forge/CI.yml?label=CI
[CI URL]: https://github.com/amacati/pixel_forge/actions/workflows/CI.yml

[Version]: https://img.shields.io/pypi/v/pixel-forge
[Version URL]: https://pypi.org/project/pixel-forge/

[Python Version]: https://img.shields.io/pypi/pyversions/pixel-forge?logo=python
[Python Version URL]: https://www.python.org

[Rust]: https://img.shields.io/badge/Rust-x?logo=rust&color=CE412B
[Rust URL]: https://www.rust-lang.org/

[Docs Status]: https://img.shields.io/github/actions/workflow/status/amacati/pixel_forge/docs.yml?label=docs
[Docs URL]: https://amacati.github.io/pixel_forge/

<p align="center">
  <img width="256" height="256" src="https://raw.githubusercontent.com/amacati/pixel_forge/master/docs/img/pixel_forge_banner.png">
</p>

Ultra-fast screen capture in Rust with Python bindings.

## Installing

The package can be installed via

```bash
pip install pixel_forge
```

Wheels are provided for Windows and Linux (x86_64).

## Platform support

The capture backend is selected automatically at build time.

- **Windows**: capture through Windows Graphics Capture.
- **Linux**: capture through X11. Wayland support through XWayland.

## Example

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

## Documentation
For a more detailed description of the package, visit our [documentation][Docs URL].

## Credit
This package started as a rewrite of [windows-capture](https://github.com/NiiightmareXD/windows-capture), and a good chunk of the implementation is taken directly from that project. Full credit goes to [NiiightmareXD](https://github.com/NiiightmareXD) for those parts.