"""protochem tests."""

import protochem


def test_stub() -> None:
    """Stub test to ensure the test suite runs."""
    print(protochem.__version__)  # noqa: T201


def test__greet() -> None:
    """Test the greet function."""
    assert protochem.greet("World") == "Hello, World!"
