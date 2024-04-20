.. _install:

Installation
============
To install ``pixel_forge``, you can use pip:

.. code-block:: bash

    pip install pixel_forge

This will install the Python package with the precompiled library without requiring Rust to be installed on your system.

.. warning::
    ``pixel_forge`` is currently only available for Windows!

Building from Source
--------------------
Alternatively, you can build the package from source by cloning the repository and compile it with pip:

.. note::
    This step requires Rust to be installed on your system.

.. code-block:: bash

    git clone https://github.com/amacati/pixel_forge
    cd pixel_forge
    pip install .

If you just want to compile the Rust package without directly installing it in Python, you can use Rust's Cargo:

.. code-block:: bash

    git clone https://github.com/amacati/pixel_forge
    cd pixel_forge
    cargo build --release
