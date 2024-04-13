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
    updates the frame pointer. Only when ``frame`` is called, the frame is materialized
    into a numpy array.

    Note:
        The capture thread is started and stopped using the ``start`` and ``stop`` methods. If the
        capture thread has not been started or no frame has arrived yet, the ``frame`` method will
        raise an error. To prevent this, you can use the ``await_first_frame`` parameter to wait for
        the first frame to arrive before returning from the ``start`` method.
    """

    def start(self, capture_target: Monitor | Window, await_first_frame: bool = False) -> None:
        """Start the capture thread.

        Args:
            capture_target: The monitor or window to capture.
            await_first_frame: Waits for the first frame to arrive if True.
        """

    def stop(self) -> None:
        """Stop the capture thread."""

    def frame(self) -> np.ndarray:
        """Materialize the current frame textures into an array and return it.

        Returns:
            The current frame as a numpy array.

        Raises:
            RuntimeError: If the capture thread has not yet picked up a frame.
        """
