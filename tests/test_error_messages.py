# ============================================================================
# File: /tests/test_error_messages.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the error message catalog module.
# Verifies that exception types are mapped to user-friendly messages
# with correct resolution priority (exact match > MRO walk > type-only > generic).
# ============================================================================

import pytest                                              # Test framework

from utils.error_messages import (
    get_user_friendly_message,                             # Main lookup function
    ErrorMessage,                                          # Message dataclass
    _GENERIC_FALLBACK,                                     # Generic fallback constant
)


# ============================================================================
# Tests: ErrorMessage dataclass
# ============================================================================

class TestErrorMessageDataclass:
    """Tests for the ErrorMessage frozen dataclass."""

    def test_has_required_fields(self):
        """ErrorMessage should have headline, explanation, and suggestion."""
        msg = ErrorMessage(
            headline="Test Headline",
            explanation="Test explanation.",
            suggestion="Test suggestion.",
        )
        assert msg.headline == "Test Headline"
        assert msg.explanation == "Test explanation."
        assert msg.suggestion == "Test suggestion."

    def test_is_frozen(self):
        """ErrorMessage should be immutable (frozen=True)."""
        msg = ErrorMessage(
            headline="Test", explanation="Test", suggestion="Test",
        )
        with pytest.raises(AttributeError):
            msg.headline = "Modified"


# ============================================================================
# Tests: Exact match resolution
# ============================================================================

class TestExactMatchResolution:
    """Tests for exact (type, context) match lookups."""

    def test_file_not_found_scan_context(self):
        """FileNotFoundError with 'scan' context should return scan-specific message."""
        exc = FileNotFoundError("test.mp3")
        msg = get_user_friendly_message(exc, context="scan")
        assert "Media file not found" in msg.headline

    def test_file_not_found_metadata_context(self):
        """FileNotFoundError with 'metadata' context should return metadata-specific message."""
        exc = FileNotFoundError("test.mp3")
        msg = get_user_friendly_message(exc, context="metadata")
        assert "File not found" in msg.headline

    def test_connection_error_lookup_context(self):
        """ConnectionError with 'lookup' context should return lookup-specific message."""
        exc = ConnectionError("connection refused")
        msg = get_user_friendly_message(exc, context="lookup")
        assert "metadata provider" in msg.headline.lower() or "connect" in msg.headline.lower()

    def test_timeout_error_lookup_context(self):
        """TimeoutError with 'lookup' context should return timeout-specific message."""
        exc = TimeoutError("timed out")
        msg = get_user_friendly_message(exc, context="lookup")
        assert "timed out" in msg.headline.lower()


# ============================================================================
# Tests: Type-only fallback resolution
# ============================================================================

class TestTypeOnlyFallback:
    """Tests for type-only (no context) fallback lookups."""

    def test_permission_error_no_context(self):
        """PermissionError with no context should return type-only message."""
        exc = PermissionError("access denied")
        msg = get_user_friendly_message(exc)
        assert "Permission denied" in msg.headline

    def test_os_error_no_context(self):
        """OSError with no context should return type-only message."""
        exc = OSError("disk full")
        msg = get_user_friendly_message(exc)
        assert "File system error" in msg.headline

    def test_connection_error_no_context(self):
        """ConnectionError with no context should fall back to type-only message."""
        exc = ConnectionError("reset")
        msg = get_user_friendly_message(exc)
        assert "Network" in msg.headline or "connection" in msg.headline.lower()


# ============================================================================
# Tests: MRO (inheritance) resolution
# ============================================================================

class TestMROResolution:
    """Tests for MRO-based fallback when subclass has no direct catalog entry."""

    def test_subclass_of_permission_error(self):
        """A PermissionError subclass should resolve via MRO to PermissionError entry."""
        class CustomPermissionError(PermissionError):
            pass
        exc = CustomPermissionError("custom denial")
        msg = get_user_friendly_message(exc)
        assert "Permission denied" in msg.headline

    def test_subclass_of_os_error(self):
        """FileExistsError (subclass of OSError) should fall back to OSError entry."""
        exc = FileExistsError("already exists")
        msg = get_user_friendly_message(exc)
        # Should match OSError type-only entry (FileExistsError -> OSError -> Exception)
        assert isinstance(msg, ErrorMessage)
        assert msg.headline  # Not empty

    def test_permission_error_inherits_os_error(self):
        """PermissionError is a subclass of OSError; its own entry should take priority."""
        exc = PermissionError("no access")
        msg = get_user_friendly_message(exc)
        # PermissionError has its own entry, which should take priority over OSError
        assert "Permission" in msg.headline


# ============================================================================
# Tests: Generic fallback
# ============================================================================

class TestGenericFallback:
    """Tests for the generic fallback when no catalog entry matches."""

    def test_unknown_exception_returns_generic(self):
        """An unknown exception type should return the generic fallback."""
        class VeryCustomError(Exception):
            pass
        exc = VeryCustomError("something weird")
        msg = get_user_friendly_message(exc)
        assert msg == _GENERIC_FALLBACK
        assert "unexpected" in msg.headline.lower()

    def test_generic_has_all_fields(self):
        """The generic fallback should have non-empty headline, explanation, suggestion."""
        assert _GENERIC_FALLBACK.headline
        assert _GENERIC_FALLBACK.explanation
        assert _GENERIC_FALLBACK.suggestion

    def test_unknown_context_falls_back_to_type(self):
        """PermissionError with unknown context should fall back to type-only."""
        exc = PermissionError("denied")
        msg = get_user_friendly_message(exc, context="nonexistent_context")
        assert "Permission denied" in msg.headline


# ============================================================================
# Tests: Return type consistency
# ============================================================================

class TestReturnTypeConsistency:
    """Verify that all resolution paths return an ErrorMessage."""

    def test_returns_error_message_for_known_type(self):
        """Should return an ErrorMessage for a known exception type."""
        msg = get_user_friendly_message(PermissionError("test"))
        assert isinstance(msg, ErrorMessage)

    def test_returns_error_message_for_unknown_type(self):
        """Should return an ErrorMessage for an unknown exception type."""
        msg = get_user_friendly_message(Exception("test"))
        assert isinstance(msg, ErrorMessage)

    def test_returns_error_message_with_context(self):
        """Should return an ErrorMessage when context is provided."""
        msg = get_user_friendly_message(FileNotFoundError("test"), context="scan")
        assert isinstance(msg, ErrorMessage)

    def test_suggestion_is_actionable(self):
        """Suggestions should contain actionable language."""
        exc = PermissionError("denied")
        msg = get_user_friendly_message(exc)
        # The suggestion should contain verbs or instructions
        assert any(word in msg.suggestion.lower() for word in
                   ["check", "try", "open", "verify", "ensure", "grant"])
