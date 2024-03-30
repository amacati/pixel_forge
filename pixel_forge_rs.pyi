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

    def valid(self) -> bool:
        """Check if the window is still valid.

        Returns:
            True if the window is valid, False otherwise.
        """

    def title(self) -> str:
        """Get the window title.

        Returns:
            The window title.
        """
