"""Capture the first available window and display the frame.

    python examples/capture_window.py

Requires matplotlib for display (``pip install matplotlib``).
"""

import matplotlib.pyplot as plt
import pixel_forge as pf


def main():
    """Capture the first available window and display the frame."""
    windows = pf.enumerate_windows()  # List the top-level windows.
    if not windows:
        raise RuntimeError("No windows found")

    window = windows[0]  # Take the first window as the capture target.
    print(f"Capturing window {window.name!r}")

    # Start a capture on the window, grab one frame, then release the resources.
    capture = pf.Capture()
    capture.start(window)
    frame = capture.frame()
    capture.stop()

    # frame is an (height, width, 4) uint8 RGBA array that matplotlib shows directly.
    plt.imshow(frame)
    plt.title(window.name)
    plt.axis("off")
    plt.show()


if __name__ == "__main__":
    main()
