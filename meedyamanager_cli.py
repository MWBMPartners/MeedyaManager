# ============================================================================
# File: /meedyamanager_cli.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Root entry script for the MeedyaManager CLI/service application.
# Used as the Nuitka build target for the standalone CLI binary.
#
# Usage:
#   python meedyamanager_cli.py scan            # Run directly
#   nuitka --standalone meedyamanager_cli.py    # Build with Nuitka
# ============================================================================

from cli import cli                                        # Click CLI entry point

if __name__ == "__main__":
    cli()
