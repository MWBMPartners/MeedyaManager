# ============================================================================
# File: /cli/runner.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Legacy entry point for the MeedyaManager batch scanner.
# Now delegates to the Click-based scan command.
# Kept for backward compatibility: `python cli/runner.py` still works.
# ============================================================================

from cli import cli                                    # Click CLI group

def main():
    """Legacy entry point — delegates to the Click CLI."""
    cli(["scan"])

if __name__ == "__main__":
    main()
