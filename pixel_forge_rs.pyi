import numpy as np

def enumerate_windows() -> list[Window]:
    """Create a list of all windows.

    Returns:
        The list of all windows.
    """

def foreground_window() -> Window:
    """Get the current foreground window.

    Returns:
        The window.
    """

class Window:
    """Window abstraction for Windows."""

    def __init__(self, name: str):
        """Create a new window object.

        Args:
            name: The window name.
        """

    @property
    def valid(self) -> bool:
        """Check if the window is still valid."""

    @property
    def title(self) -> str:
        """The window title."""

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
    """Monitor abstraction for Windows."""

    @staticmethod
    def primary() -> Monitor:
        """Get the primary monitor.

        Returns:
            The primary monitor.
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
    """Capture class encapsulating the screen capture functionality.

    The screen recording is done in a separate thread. Each time a frame is produced, the thread
    updates the frame pointer. Only when ``materialize_frame`` is called, the frame is materialized
    into a numpy array.

    Note:
        The capture thread is started and stopped using the ``start_capture_thread`` and
        ``stop_capture_thread`` methods. If the capture thread has not been started or no frame has
        arrived yet, the ``materialize_frame`` method will return an empty array.
    """

    def start(self):
        """Start the capture thread."""

    def stop(self):
        """Stop the capture thread."""

    def materialize_frame(self) -> np.ndarray:
        """Materialize the current frame.

        Warning:
            If the capture thread has not been started or no frame has arrived yet, this method will
            return an empty array.

        Returns:
            The current frame as a numpy array.
        """
