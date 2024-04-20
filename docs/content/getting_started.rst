.. _getting_started:

Getting Started
===============
``pixel_forge`` provides a minimalistic interface to capture a screenshot of the screen. You can capture a screenshot of your primary monitor as a NumPy array by running

.. code-block:: python

    from pixel_forge import Capture, Monitor

    m = Monitor()
    c = Capture()
    c.start(m)
    img = c.frame()

.. warning::
    ``pixel_forge`` is currently only available for Windows! This code will not work on other operating systems.

You can also capture a screenshot of a specific monitor:

.. code-block:: python

    from pixel_forge import Capture, enumerate_monitors

    m = enumerate_monitors()[1]  # Capture the second monitor
    c = Capture()
    c.start(m)
    img = c.frame()
    c.stop()  # Explicitly stop the capture. Will also happen when the object is garbage collected

Each call of ``frame()`` will fetch the latest available frame. You can also explicitly stop the capture by calling ``stop()``. This will release the resources used by the capture process.
If you want to capture a specific window, you can do so as well:

.. code-block:: python

    from pixel_forge import Capture, Window

    w = Window("Untitled - Notepad")  # Assuming you have an untitled Notepad window open
    c = Capture()
    c.start(w)
    img = c.frame()

