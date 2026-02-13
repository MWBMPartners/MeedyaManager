# ============================================================================
# File: /tests/test_credentials.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CredentialManager 4-tier priority chain:
# - Environment variable resolution (tier 1)
# - Config file resolution (tier 2)
# - OS keyring resolution (tier 3, mocked)
# - Encrypted bundle resolution (tier 4, mocked)
# - Priority ordering (env > config > keyring > bundle)
# - has_credentials() for various providers
# - Credential field definitions for all 19 providers
# ============================================================================

import os                                                  # Environment variable manipulation
import pytest                                              # Test framework
from unittest.mock import patch, MagicMock                 # Mocking for keyring/config

from metadata.providers.credentials import (
    CredentialManager,                                     # Main class under test
    CREDENTIAL_FIELDS,                                     # Provider credential definitions
)


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def cred_manager():
    """Create a fresh CredentialManager instance."""
    return CredentialManager()


# =============================================================================
# CREDENTIAL_FIELDS Structure Tests
# =============================================================================

class TestCredentialFields:
    """Tests for the CREDENTIAL_FIELDS mapping."""

    def test_all_19_providers_defined(self):
        """All 19 providers should have entries in CREDENTIAL_FIELDS."""
        expected_providers = {
            "apple_music", "spotify", "musicbrainz", "deezer",
            "youtube_music", "amazon_music", "pandora", "tidal",
            "shazam", "iheart", "tmdb", "tvdb", "imdb",
            "apple_tv", "itunes_store", "apple_podcasts",
            "isrc", "eidr", "iswc",
        }
        actual_providers = set(CREDENTIAL_FIELDS.keys())
        assert expected_providers == actual_providers

    def test_spotify_requires_two_fields(self):
        """Spotify should require client_id and client_secret."""
        fields = CREDENTIAL_FIELDS["spotify"]
        field_names = [f[0] for f in fields]
        assert "client_id" in field_names
        assert "client_secret" in field_names

    def test_apple_music_requires_three_fields(self):
        """Apple Music should require team_id, key_id, and private_key."""
        fields = CREDENTIAL_FIELDS["apple_music"]
        field_names = [f[0] for f in fields]
        assert "team_id" in field_names
        assert "key_id" in field_names
        assert "private_key" in field_names

    def test_musicbrainz_no_credentials(self):
        """MusicBrainz should require no credentials."""
        assert CREDENTIAL_FIELDS["musicbrainz"] == []

    def test_deezer_no_credentials(self):
        """Deezer search should require no credentials."""
        assert CREDENTIAL_FIELDS["deezer"] == []

    def test_eidr_requires_credentials(self):
        """EIDR should require client_id and client_secret."""
        fields = CREDENTIAL_FIELDS["eidr"]
        field_names = [f[0] for f in fields]
        assert "client_id" in field_names
        assert "client_secret" in field_names


# =============================================================================
# Environment Variable Resolution (Tier 1) Tests
# =============================================================================

class TestEnvResolution:
    """Tests for Tier 1: Environment variable credential resolution."""

    def test_env_var_found(self, cred_manager):
        """Credential should be found when the env var is set."""
        with patch.dict(os.environ, {"SPOTIFY_CLIENT_ID": "test_id_123"}):
            value = cred_manager.get_credential("spotify", "client_id")
            assert value == "test_id_123"

    def test_env_var_empty_returns_none(self, cred_manager):
        """Empty env var should be treated as unset (return None)."""
        with patch.dict(os.environ, {"SPOTIFY_CLIENT_ID": ""}):
            value = cred_manager._get_from_env("spotify", "client_id")
            assert value is None

    def test_env_var_not_set_returns_none(self, cred_manager):
        """Missing env var should return None."""
        with patch.dict(os.environ, {}, clear=True):
            value = cred_manager._get_from_env("spotify", "client_id")
            assert value is None


# =============================================================================
# Config File Resolution (Tier 2) Tests
# =============================================================================

class TestConfigResolution:
    """Tests for Tier 2: Config file credential resolution."""

    def test_config_found(self, cred_manager):
        """Credential should be found in config when providers section exists."""
        mock_config = {
            "providers": {
                "tmdb": {"api_key": "config_tmdb_key_123"}
            }
        }
        cred_manager._config_cache = mock_config
        value = cred_manager._get_from_config("tmdb", "api_key")
        assert value == "config_tmdb_key_123"

    def test_config_missing_provider(self, cred_manager):
        """Missing provider section should return None."""
        cred_manager._config_cache = {"providers": {}}
        value = cred_manager._get_from_config("spotify", "client_id")
        assert value is None

    def test_config_missing_field(self, cred_manager):
        """Missing field in provider section should return None."""
        cred_manager._config_cache = {
            "providers": {"spotify": {"client_id": "abc"}}
        }
        value = cred_manager._get_from_config("spotify", "client_secret")
        assert value is None


# =============================================================================
# Keyring Resolution (Tier 3) Tests
# =============================================================================

class TestKeyringResolution:
    """Tests for Tier 3: OS keyring credential resolution."""

    def test_keyring_found(self, cred_manager):
        """Credential should be found when keyring has the value."""
        mock_keyring = MagicMock()
        mock_keyring.get_password.return_value = "keyring_secret_123"
        with patch.dict("sys.modules", {"keyring": mock_keyring}):
            value = cred_manager._get_from_keyring("spotify", "client_secret")
            assert value == "keyring_secret_123"

    def test_keyring_not_installed(self, cred_manager):
        """Missing keyring package should return None gracefully."""
        with patch.dict("sys.modules", {"keyring": None}):
            # Force ImportError by removing keyring from available modules
            with patch("builtins.__import__", side_effect=ImportError("No keyring")):
                value = cred_manager._get_from_keyring("spotify", "client_secret")
                assert value is None


# =============================================================================
# Priority Chain Tests
# =============================================================================

class TestPriorityChain:
    """Tests for credential priority ordering (env > config > keyring > bundle)."""

    def test_env_takes_priority_over_config(self, cred_manager):
        """Environment variable should override config file."""
        cred_manager._config_cache = {
            "providers": {"spotify": {"client_id": "config_value"}}
        }
        with patch.dict(os.environ, {"SPOTIFY_CLIENT_ID": "env_value"}):
            value = cred_manager.get_credential("spotify", "client_id")
            assert value == "env_value"

    def test_config_used_when_no_env(self, cred_manager):
        """Config value should be used when env var is not set."""
        cred_manager._config_cache = {
            "providers": {"tmdb": {"api_key": "config_key"}}
        }
        # Ensure env var is not set
        env = {k: v for k, v in os.environ.items() if k != "TMDB_API_KEY"}
        with patch.dict(os.environ, env, clear=True):
            value = cred_manager.get_credential("tmdb", "api_key")
            assert value == "config_key"


# =============================================================================
# has_credentials() Tests
# =============================================================================

class TestHasCredentials:
    """Tests for the has_credentials() method."""

    def test_no_credentials_required(self, cred_manager):
        """Providers with no required fields should always return True."""
        assert cred_manager.has_credentials("musicbrainz") is True
        assert cred_manager.has_credentials("deezer") is True
        assert cred_manager.has_credentials("shazam") is True
        assert cred_manager.has_credentials("imdb") is True

    def test_credentials_present(self, cred_manager):
        """Should return True when all required credentials are available."""
        with patch.dict(os.environ, {
            "SPOTIFY_CLIENT_ID": "test_id",
            "SPOTIFY_CLIENT_SECRET": "test_secret",
        }):
            assert cred_manager.has_credentials("spotify") is True

    def test_credentials_partial(self, cred_manager):
        """Should return False when only some credentials are available."""
        cred_manager._config_cache = {"providers": {}}
        env = {k: v for k, v in os.environ.items()
               if k not in ("SPOTIFY_CLIENT_ID", "SPOTIFY_CLIENT_SECRET")}
        env["SPOTIFY_CLIENT_ID"] = "test_id"               # Only one of two
        with patch.dict(os.environ, env, clear=True):
            # Ensure keyring and bundle don't provide the secret
            with patch.object(cred_manager, "_get_from_keyring", return_value=None):
                with patch.object(cred_manager, "_get_from_bundle", return_value=None):
                    assert cred_manager.has_credentials("spotify") is False

    def test_unknown_provider(self, cred_manager):
        """Unknown provider (not in CREDENTIAL_FIELDS) should return True."""
        # Empty list defaults to no credentials needed
        assert cred_manager.has_credentials("unknown_provider") is True


# =============================================================================
# Utility Method Tests
# =============================================================================

class TestUtilityMethods:
    """Tests for helper methods."""

    def test_get_env_var_name_known(self, cred_manager):
        """Should return the correct env var name for known credentials."""
        assert cred_manager.get_env_var_name("spotify", "client_id") == "SPOTIFY_CLIENT_ID"
        assert cred_manager.get_env_var_name("tmdb", "api_key") == "TMDB_API_KEY"
        assert cred_manager.get_env_var_name("eidr", "client_secret") == "EIDR_CLIENT_SECRET"

    def test_get_env_var_name_fallback(self, cred_manager):
        """Unknown field should construct an env var name from provider/field."""
        result = cred_manager.get_env_var_name("unknown", "some_field")
        assert result == "UNKNOWN_SOME_FIELD"

    def test_get_required_fields_spotify(self, cred_manager):
        """Should return the field tuples for Spotify."""
        fields = cred_manager.get_required_fields("spotify")
        assert len(fields) == 2
        field_names = [f[0] for f in fields]
        assert "client_id" in field_names
        assert "client_secret" in field_names

    def test_get_required_fields_empty(self, cred_manager):
        """Providers with no credentials should return empty list."""
        fields = cred_manager.get_required_fields("musicbrainz")
        assert fields == []

    def test_set_user_credential_keyring_not_installed(self, cred_manager):
        """set_user_credential() should return False when keyring is not installed."""
        with patch("builtins.__import__", side_effect=ImportError("No keyring")):
            result = cred_manager.set_user_credential("spotify", "client_id", "test")
            assert result is False
