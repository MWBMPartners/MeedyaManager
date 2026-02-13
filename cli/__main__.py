# ============================================================================
# File: /cli/__main__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Enables running the CLI as a Python module: `python -m cli [COMMAND]`
# This is the recommended way to invoke MeedyaManager from the command line.
# ============================================================================

from cli import cli                                    # Click CLI group

if __name__ == "__main__":
    cli()
