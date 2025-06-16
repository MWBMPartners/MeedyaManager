# ============================================================================
# File: /utils/env_loader.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Loads .env variables from a .env file into the environment using dotenv.
# This enables fallback credential loading for dev/test environments.
# ============================================================================

import os
from pathlib import Path
from dotenv import load_dotenv


def load_env_variables(dotenv_path: str = ".env"):
    """
    Load environment variables from a .env file if present.
    These are used for dev/local testing and fallback credentials.
    """
    env_file = Path(dotenv_path).expanduser().resolve()
    if env_file.exists():
        load_dotenv(dotenv_path)
        print(f"[env_loader] Loaded environment variables from: {env_file}")
    else:
        print("[env_loader] No .env file found (skipped)")


if __name__ == "__main__":
    load_env_variables()