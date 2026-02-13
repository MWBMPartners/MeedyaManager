# ============================================================================
# File: /cli/metadata_debugger.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Legacy entry point for the MeedyaManager metadata debugger.
# Now delegates to the Click-based debug command.
# Kept for backward compatibility: `python cli/metadata_debugger.py <file>` still works.
# ============================================================================

import sys                                             # Access command-line arguments
from cli.commands.debug import debug                   # Click debug command


def main():
    """Legacy entry point — delegates to the Click debug command."""
    debug()


if __name__ == "__main__":
    main()
