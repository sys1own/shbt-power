"""Module entry point: `python3 -m shbt_power run|audit`."""

import sys

from shbt_power.cli import main

if __name__ == "__main__":
    sys.exit(main())
