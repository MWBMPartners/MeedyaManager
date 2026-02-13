# ============================================================================
# File: /meedyamanager_gui.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Root entry script for the MeedyaManager GUI application.
# Used as the Nuitka build target for the standalone GUI binary.
#
# Usage:
#   python meedyamanager_gui.py          # Run directly
#   nuitka --standalone meedyamanager_gui.py  # Build with Nuitka
# ============================================================================

import sys                                                 # System exit codes
from ui.app import launch_gui                              # GUI launcher function

if __name__ == "__main__":
    sys.exit(launch_gui())
