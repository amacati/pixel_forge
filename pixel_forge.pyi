from __future__ import annotations

import numpy as np

def enumerate_windows() -> list[Window]:
    """Create a list of all windows that are currently available.

    Returns:
        The list of all windows.

    Raises:
        RuntimeError: If the window enumeration fails.
    """

def foreground_window() -> Window:
    """Get the current foreground window.

    Returns:
        The foreground window.

    Raises:
        RuntimeError: No foreground window was found.
    """

class Window:
    """Window abstraction for the Windows operating system.

    Windows can be used as capture target for the :class:`.Capture` class.
    """

    def __init__(self, name: str):
        """Create a new window object.

        Args:
            name: The window name.
        """

    @property
    def valid(self) -> bool:
        """True if the window is still valid (i.e., open), else False."""

    @property
    def name(self) -> str:
        """The window name."""

def primary_monitor() -> Monitor:
    """Get the primary monitor.

    Returns:
        The primary monitor.
    """

def enumerate_monitors() -> list[Monitor]:
    """Create a list of all monitors.

    Returns:
        The list of all monitors.
    """

class Monitor:
    """Monitor abstraction the Windows operating system."""

    def __init__(self, id: int | None = None):
        """Create a new monitor object.

        Args:
            id: The monitor ID. If None, the primary monitor is selected. Monitor IDs start at 1.
        """

    @property
    def width(self) -> int:
        """The monitor pixel width."""

    @property
    def height(self) -> int:
        """The monitor pixel height."""

    @property
    def index(self) -> int:
        """The monitor index."""

    @property
    def refresh_rate(self) -> int:
        """The monitor refresh rate in Hz."""

    @property
    def device_name(self) -> str:
        """The monitor device name."""

    @property
    def device_string(self) -> str:
        """The monitor device string."""

class Capture:
    """Capture class to capture frames from a monitor or a window.

    The idea is to get either a :class:`.Monitor` or a :class:`.Window` as target, create a Capture
    object, and then start a capture thread that will update the internal frame of the Capture
    object whenever a new frame is available. Frames are only materialized, converted to NumPy
    arrays and passed over to Python when the user requests it to avoid unnecessary copies.
    """

    def start(self, capture_target: Monitor | Window, await_first_frame: bool = True) -> None:
        """Start the capture.

        This registeres an event handler that automatically updates the latest frame whenever a new
        frame is available. The frame can be accessed using :meth:`frame`. Since the event handler
        runs in a separate thread, the first frame might not be available immediately. To ensure a
        frame is available before continuing, set ``await_first_frame`` to True. This will block the
        main thread until the first frame is available.

        Args:
            capture_target: The monitor or window to capture.
            await_first_frame: Waits for the first frame to arrive if True.
        """

    def stop(self) -> None:
        """Stop the capture thread, wait for it to join and invalidate the last frame.

        This method is also called automatically when the object is garbage collected.
        """

    def frame(self) -> np.ndarray:
        """Convert the latest frame to an array and return it.

        Returns:
            The frame as a 3D NumPy array with dimensions [h w 4] (height x width x RGBA).

        Raises:
            RuntimeError: If the capture thread has not yet picked up a frame.
        """

    @property
    def active(self) -> bool:
        """True if the capture thread is running, False otherwise."""
