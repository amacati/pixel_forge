"""Capture the primary monitor and display the frame.

    python examples/capture_monitor.py

Requires matplotlib for display (``pip install matplotlib``).
"""

import matplotlib.pyplot as plt
import pixel_forge as pf


def main():
    """Capture the primary monitor and display the frame."""
    monitor = pf.primary_monitor()  # Take the primary monitor as the capture target.
    print(f"Capturing monitor {monitor.index} ({monitor.device_name}, {monitor.width}x{monitor.height})")

    # Start a capture on the monitor, grab one frame, then release the resources.
    capture = pf.Capture()
    capture.start(monitor)
    frame = capture.frame()
    capture.stop()

    # frame is an (height, width, 4) uint8 RGBA array that matplotlib shows directly.
    plt.imshow(frame)
    plt.title(f"Monitor {monitor.index}")
    plt.axis("off")
    plt.show()


if __name__ == "__main__":
    main()
