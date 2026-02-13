# ============================================================================
# File: /utils/log_config.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Centralized logging configuration for MeedyaManager.
#
# Provides a single setup_logging() function called once at application
# startup (from CLI entry point or GUI launcher). All modules using
# logging.getLogger("MeedyaManager.*") inherit this configuration.
#
# Features:
#   - Platform-aware log directory resolution (macOS, Windows, Linux)
#   - PII redaction filter (removes usernames from all log output)
#   - Daily log rotation (TimedRotatingFileHandler, 30 backups)
#   - Size-based safety net (RotatingFileHandler, 10 MB, 5 backups)
#   - Console handler at WARNING level (avoids flooding terminal)
#   - Old log cleanup on startup (configurable max age)
#   - Log level resolved from: env var → config → default "INFO"
#
# Usage:
#   from utils.log_config import setup_logging
#   setup_logging()  # Call once at app startup
# ============================================================================

import os                                              # Path and environment operations
import sys                                             # Platform detection
import re                                              # PII redaction patterns
import logging                                         # Python logging framework
from logging.handlers import (
    TimedRotatingFileHandler,                          # Daily log rotation
    RotatingFileHandler,                               # Size-based rotation safety net
)
from pathlib import Path                               # Modern path operations
from datetime import datetime, timedelta               # Log cleanup age calculation


# ============================================================================
# Constants
# ============================================================================

# Application name used in log directory paths and log format
APP_NAME = "MeedyaManager"

# Default log format — includes logger name for grep-ability
LOG_FORMAT = "[%(asctime)s] %(name)s %(levelname)s %(message)s"

# Console-only format (shorter, no logger name since context is obvious)
CONSOLE_FORMAT = "[%(asctime)s] %(levelname)s %(message)s"

# Default settings if not overridden by config or env var
DEFAULT_LOG_LEVEL = "INFO"
DEFAULT_CONSOLE_LEVEL = "WARNING"
DEFAULT_MAX_LOG_DAYS = 30
DEFAULT_MAX_LOG_SIZE_MB = 10
DEFAULT_ROTATION_BACKUPS = 5


# ============================================================================
# PII Redaction Filter
# ============================================================================

class PIIRedactionFilter(logging.Filter):
    """
    Logging filter that replaces user-identifiable path segments in log
    messages with <user> for privacy safety.

    Applied globally to all handlers so that no module needs to call a
    redaction function manually. Covers:
      - macOS:   /Users/username   → /Users/<user>
      - Windows: C:\\Users\\username → C:\\Users\\<user>
      - Linux:   /home/username    → /home/<user>

    The filter mutates the LogRecord in-place before the handler formats it.
    """

    # Compiled regex patterns for user path detection
    PATTERNS = [
        re.compile(r"/Users/\w+"),                     # macOS user directories
        re.compile(r"/home/\w+"),                      # Linux user directories
        re.compile(r'C:\\Users\\[^\\/\s"]+'),          # Windows user directories
    ]

    def filter(self, record):
        """
        Redact PII from the log record message and arguments.

        Args:
            record: The LogRecord to filter.

        Returns:
            True always (the record is never dropped, only sanitized).
        """
        # Redact the raw message string
        if isinstance(record.msg, str):
            for pattern in self.PATTERNS:
                record.msg = pattern.sub(
                    lambda m: m.group(0).rsplit("/", 1)[0] + "/<user>"
                    if "/" in m.group(0)
                    else m.group(0).rsplit("\\", 1)[0] + "\\<user>",
                    record.msg,
                )

        # If the record has formatting args, redact the fully formatted message
        # and clear args to prevent double-formatting
        if record.args:
            try:
                formatted = record.getMessage()
                for pattern in self.PATTERNS:
                    formatted = pattern.sub(
                        lambda m: m.group(0).rsplit("/", 1)[0] + "/<user>"
                        if "/" in m.group(0)
                        else m.group(0).rsplit("\\", 1)[0] + "\\<user>",
                        formatted,
                    )
                record.msg = formatted
                record.args = None
            except Exception:
                pass                                   # Don't crash logging over redaction

        return True


# ============================================================================
# Log Directory Resolution
# ============================================================================

def get_log_directory() -> Path:
    """
    Resolve the platform-appropriate log directory for MeedyaManager.

    Platform conventions:
      - macOS:   ~/Library/Logs/MeedyaManager/
      - Windows: %LOCALAPPDATA%/MeedyaManager/logs/
      - Linux:   ~/.local/state/MeedyaManager/logs/

    Falls back to a logs/ directory relative to the project root if
    the platform-specific directory cannot be created (e.g., permissions).

    Returns:
        Path: Absolute path to the log directory (created if needed).
    """
    if sys.platform == "darwin":
        # macOS: ~/Library/Logs/ is the standard log location
        base = Path.home() / "Library" / "Logs" / APP_NAME
    elif sys.platform == "win32":
        # Windows: %LOCALAPPDATA%/AppName/logs/ is conventional
        local_app = os.environ.get(
            "LOCALAPPDATA",
            str(Path.home() / "AppData" / "Local"),
        )
        base = Path(local_app) / APP_NAME / "logs"
    else:
        # Linux and other POSIX: ~/.local/state/ for runtime logs (XDG)
        base = Path.home() / ".local" / "state" / APP_NAME / "logs"

    try:
        base.mkdir(parents=True, exist_ok=True)
        return base
    except OSError:
        # Fallback: logs/ directory relative to the utils/ module
        fallback = Path(__file__).resolve().parent.parent / "logs"
        fallback.mkdir(parents=True, exist_ok=True)
        return fallback


# ============================================================================
# Old Log Cleanup
# ============================================================================

def cleanup_old_logs(log_dir: Path, max_age_days: int = DEFAULT_MAX_LOG_DAYS):
    """
    Remove log files older than max_age_days from the log directory.

    Targets files matching patterns:
      - meedyamanager.log.*    (rotated daily log files)
      - crash_*.txt            (crash report files)

    This prevents unbounded disk usage over time. Called during
    setup_logging() at application startup.

    Args:
        log_dir: Path to the log directory to clean.
        max_age_days: Maximum age in days before a log file is deleted.
    """
    cutoff = datetime.now() - timedelta(days=max_age_days)

    # Clean rotated log files
    for pattern in ["meedyamanager.log.*", "crash_*.txt"]:
        for log_file in log_dir.glob(pattern):
            try:
                file_mtime = datetime.fromtimestamp(log_file.stat().st_mtime)
                if file_mtime < cutoff:
                    log_file.unlink(missing_ok=True)
            except OSError:
                pass                                   # Skip files that can't be stat'd/deleted


# ============================================================================
# Log Level Resolution
# ============================================================================

def _resolve_log_level(override: str = None) -> str:
    """
    Resolve the effective log level from the priority chain.

    Priority order (highest first):
      1. Explicit override parameter
      2. METAMANCER_LOG_LEVEL environment variable
      3. config/settings.json5 → logging.level
      4. Default: "INFO"

    Args:
        override: Explicit log level string. If provided, takes top priority.

    Returns:
        str: A valid Python logging level name (DEBUG, INFO, WARNING, etc.)
    """
    if override:
        return override.upper()

    # Check environment variable
    env_level = os.environ.get("METAMANCER_LOG_LEVEL")
    if env_level:
        return env_level.upper()

    # Check config file (import here to avoid circular imports at module load)
    try:
        from utils.config_loader import get_config     # Lazy import
        logging_config = get_config("logging", default={})
        if isinstance(logging_config, dict) and "level" in logging_config:
            return str(logging_config["level"]).upper()
    except Exception:
        pass                                           # Config not available yet

    return DEFAULT_LOG_LEVEL


def _resolve_console_level() -> str:
    """
    Resolve the console handler log level from config.

    Returns:
        str: A valid Python logging level name for console output.
    """
    try:
        from utils.config_loader import get_config     # Lazy import
        logging_config = get_config("logging", default={})
        if isinstance(logging_config, dict) and "console_level" in logging_config:
            return str(logging_config["console_level"]).upper()
    except Exception:
        pass

    return DEFAULT_CONSOLE_LEVEL


def _resolve_max_log_size_mb() -> int:
    """Resolve max log file size from config (default: 10 MB)."""
    try:
        from utils.config_loader import get_config
        logging_config = get_config("logging", default={})
        if isinstance(logging_config, dict) and "max_log_size_mb" in logging_config:
            return int(logging_config["max_log_size_mb"])
    except Exception:
        pass
    return DEFAULT_MAX_LOG_SIZE_MB


def _resolve_max_log_days() -> int:
    """Resolve max log retention days from config (default: 30)."""
    try:
        from utils.config_loader import get_config
        logging_config = get_config("logging", default={})
        if isinstance(logging_config, dict) and "max_log_days" in logging_config:
            return int(logging_config["max_log_days"])
    except Exception:
        pass
    return DEFAULT_MAX_LOG_DAYS


def _should_redact_pii() -> bool:
    """Check if PII redaction is enabled in config (default: True)."""
    try:
        from utils.config_loader import get_config
        logging_config = get_config("logging", default={})
        if isinstance(logging_config, dict) and "redact_pii" in logging_config:
            return bool(logging_config["redact_pii"])
    except Exception:
        pass
    return True


# ============================================================================
# Main Setup Function
# ============================================================================

# Track whether logging has already been configured to prevent double-setup
_logging_configured = False


def setup_logging(log_level: str = None):
    """
    Configure the centralized MeedyaManager logging system.

    Called ONCE at application startup from the CLI entry point or GUI
    launcher. Sets up the root "MeedyaManager" logger with file rotation,
    PII redaction, and console output. All child loggers (e.g.,
    "MeedyaManager.Watcher", "MeedyaManager.Renamer") inherit this config.

    Also bridges the legacy "watcher" logger (if it exists) by setting its
    parent to the MeedyaManager root logger, ensuring consistent output.

    Args:
        log_level: Override log level string (e.g., "DEBUG"). If None,
            resolved from env var → config → default "INFO".
    """
    global _logging_configured
    if _logging_configured:
        return                                         # Prevent duplicate handler setup
    _logging_configured = True

    # Resolve configuration values
    effective_level = _resolve_log_level(log_level)
    console_level = _resolve_console_level()
    max_size_mb = _resolve_max_log_size_mb()
    max_days = _resolve_max_log_days()
    redact_pii = _should_redact_pii()

    # Get or create the log directory
    log_dir = get_log_directory()
    log_file = log_dir / "meedyamanager.log"

    # Clean up old log files before creating new handlers
    cleanup_old_logs(log_dir, max_age_days=max_days)

    # ── Create the root MeedyaManager logger ──
    root_logger = logging.getLogger(APP_NAME)
    root_logger.setLevel(getattr(logging, effective_level, logging.INFO))

    # Allow propagation to the Python root logger so that pytest's caplog
    # fixture can capture MeedyaManager log messages. The Python root logger
    # typically has no handlers (unless explicitly added), so this does not
    # cause duplicate output in production. Our handlers on the MeedyaManager
    # logger handle all output.
    root_logger.propagate = True

    # ── Create formatters ──
    file_formatter = logging.Formatter(LOG_FORMAT)
    console_formatter = logging.Formatter(CONSOLE_FORMAT)

    # ── PII Redaction Filter ──
    pii_filter = PIIRedactionFilter() if redact_pii else None

    # ── Handler 1: TimedRotatingFileHandler (daily rotation) ──
    try:
        timed_handler = TimedRotatingFileHandler(
            str(log_file),
            when="midnight",                           # Rotate at midnight
            interval=1,                                # Every 1 day
            backupCount=max_days,                      # Keep N daily backups
            encoding="utf-8",
        )
        timed_handler.setFormatter(file_formatter)
        timed_handler.setLevel(getattr(logging, effective_level, logging.INFO))
        if pii_filter:
            timed_handler.addFilter(pii_filter)
        root_logger.addHandler(timed_handler)
    except OSError:
        pass                                           # Can't create file handler (read-only FS)

    # ── Handler 2: RotatingFileHandler (size-based safety net) ──
    try:
        size_handler = RotatingFileHandler(
            str(log_file),
            maxBytes=max_size_mb * 1024 * 1024,        # Convert MB to bytes
            backupCount=DEFAULT_ROTATION_BACKUPS,
            encoding="utf-8",
        )
        size_handler.setFormatter(file_formatter)
        size_handler.setLevel(getattr(logging, effective_level, logging.INFO))
        if pii_filter:
            size_handler.addFilter(pii_filter)
        root_logger.addHandler(size_handler)
    except OSError:
        pass                                           # Can't create file handler (read-only FS)

    # ── Handler 3: StreamHandler (console output) ──
    console_handler = logging.StreamHandler()
    console_handler.setFormatter(console_formatter)
    console_handler.setLevel(getattr(logging, console_level, logging.WARNING))
    if pii_filter:
        console_handler.addFilter(pii_filter)
    root_logger.addHandler(console_handler)

    # ── Bridge legacy "watcher" logger ──
    # The watcher module historically used logging.getLogger("watcher")
    # instead of "MeedyaManager.Watcher". Bridge it to the centralized config
    # by making the MeedyaManager logger its parent and disabling propagation
    # to the root Python logger.
    legacy_watcher = logging.getLogger("watcher")
    legacy_watcher.parent = root_logger
    legacy_watcher.propagate = True                    # Propagate to MeedyaManager
    # Remove any existing handlers from the legacy logger
    legacy_watcher.handlers.clear()

    root_logger.debug(
        f"Logging initialized: level={effective_level}, "
        f"log_dir={log_dir}, pii_redaction={'on' if redact_pii else 'off'}"
    )


def reset_logging():
    """
    Reset the logging configuration flag, allowing setup_logging() to be
    called again. Used primarily in tests to ensure clean logging state.
    """
    global _logging_configured
    _logging_configured = False

    # Clear handlers from the MeedyaManager root logger
    root_logger = logging.getLogger(APP_NAME)
    root_logger.handlers.clear()

    # Clear handlers from legacy watcher logger
    legacy_watcher = logging.getLogger("watcher")
    legacy_watcher.handlers.clear()
