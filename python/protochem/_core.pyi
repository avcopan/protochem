"""
The compiled Python module, imported as `<package_name>._core`.

Its type stubs (`_core.pyi`) are generated from this module by `pixi run stubs`, which
includes these doc comments as docstrings, so they use Python's (Google) docstring style.
"""

from typing import Final

__version__: Final[str]
"""
The package version.
"""

def greet(name: str) -> str:
    """
    Greet a person by their name.

    Args:
        name: The name of the person to greet.

    Returns:
        A greeting message.
    """
