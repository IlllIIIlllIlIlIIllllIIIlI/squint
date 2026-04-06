"""squint — a fast SQL linter for dbt and Jinja SQL files."""

from __future__ import annotations

import os
import shutil
import sys

__version__ = "0.2.0"
__all__ = ["main"]


def main() -> None:
    """Entry point for `python -m squint`.

    The binary is installed onto PATH by pip (in ``data/scripts/``).
    This wrapper locates it and execs, replacing the current process.
    """
    binary = shutil.which("squint")
    if binary is None:
        sys.exit(
            "squint binary not found on PATH — "
            "the installation may be incomplete; try reinstalling with pip."
        )
    os.execv(binary, [binary, *sys.argv[1:]])
