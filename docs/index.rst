pixel forge
===========

.. figure:: img/pixel_forge_banner.png
   :align: center

``pixel_forge`` is a Rust-based screenshot library for Windows with ``pyo3`` Python bindings. It is designed to be fast,
lightweight, and easy to use. Internally, ``pixel_forge`` leverages the DirectX API to fetch new frames as soon as they are available.
Frames are only materialized and copied to Python when requested.

.. toctree::
   :maxdepth: 1
   :caption: Getting Started

   content/install
   content/getting_started

.. toctree::
   :maxdepth: 1
   :caption: API Reference

   content/capture
   content/window
   content/monitor

.. toctree::
   :maxdepth: 1
   :caption: Notes

   content/acknowledgements


Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
