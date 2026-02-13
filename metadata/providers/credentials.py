# ============================================================================
# File: /metadata/providers/credentials.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Credential management system for metadata lookup providers.
# Implements a 4-tier priority chain for resolving API keys and tokens:
#
# Priority 1: Environment variables (.env file or system environment)
# Priority 2: Config file (settings.json5 → providers section)
# Priority 3: OS keyring (macOS Keychain, Windows Credential Manager, etc.)
# Priority 4: Encrypted bundle (app's own keys for ToS-compliant providers)
#
# The CredentialManager ensures that user-provided keys always take
# precedence over bundled defaults. App keys are Fernet-encrypted with
# a machine-derived key to prevent casual extraction.
# ============================================================================

import os                                           # Environment variable access
import logging                                      # Standard logging
import hashlib                                      # Key derivation for encrypted bundle
import platform                                     # Machine identification (hostname)
import uuid                                         # Machine identification (MAC address)
from pathlib import Path                            # Cross-platform path handling

logger = logging.getLogger("MeedyaManager.CredentialManager")


# ============================================================================
# CREDENTIAL_FIELDS — Defines which credentials each provider needs.
# Maps provider_name → list of (field_name, env_var_name) tuples.
# The field_name is used for config/keyring lookup; the env_var_name
# is the expected environment variable name.
# ============================================================================
CREDENTIAL_FIELDS = {
    # Music providers
    "apple_music": [
        ("team_id", "APPLE_MUSIC_TEAM_ID"),
        ("key_id", "APPLE_MUSIC_KEY_ID"),
        ("private_key", "APPLE_MUSIC_PRIVATE_KEY"),
    ],
    "spotify": [
        ("client_id", "SPOTIFY_CLIENT_ID"),
        ("client_secret", "SPOTIFY_CLIENT_SECRET"),
    ],
    "musicbrainz": [],                              # No credentials needed (User-Agent only)
    "deezer": [],                                   # No credentials needed for search
    "youtube_music": [
        ("headers_auth_path", "YOUTUBE_MUSIC_HEADERS_AUTH"),
    ],
    "amazon_music": [
        ("auth_token", "AMAZON_MUSIC_AUTH"),
    ],
    "pandora": [],                                  # No public API
    "tidal": [
        ("client_id", "TIDAL_CLIENT_ID"),
        ("client_secret", "TIDAL_CLIENT_SECRET"),
    ],
    "shazam": [],                                   # No credentials needed
    "iheart": [],                                   # No credentials needed

    # Video providers
    "tmdb": [
        ("api_key", "TMDB_API_KEY"),
    ],
    "tvdb": [
        ("api_key", "TVDB_API_KEY"),
    ],
    "imdb": [],                                     # No credentials (cinemagoer scraping)
    "apple_tv": [],                                 # No credentials (iTunes Search API)
    "itunes_store": [],                             # No credentials (public API)

    # Podcast providers
    "apple_podcasts": [],                           # No credentials (iTunes Search API)

    # Identifier providers
    "isrc": [],                                     # Federated (uses other providers)
    "eidr": [
        ("client_id", "EIDR_CLIENT_ID"),
        ("client_secret", "EIDR_CLIENT_SECRET"),
    ],
    "iswc": [],                                     # Uses MusicBrainz (no credentials)
}


class CredentialManager:
    """Manages API credentials for all metadata providers.

    Implements a 4-tier priority chain:
    1. Environment variables (e.g., SPOTIFY_CLIENT_ID from .env)
    2. Config file (settings.json5 → providers → spotify → client_id)
    3. OS keyring (macOS Keychain, Windows Credential Manager, Linux SecretService)
    4. Encrypted bundle (app's own keys, Fernet-encrypted)

    User-provided keys always take precedence over app-bundled defaults.
    """

    # Keyring service name for MeedyaManager credentials
    KEYRING_SERVICE = "meedyamanager"

    def __init__(self):
        """Initialise the credential manager."""
        self._config_cache: dict | None = None      # Cached config file data

    def get_credential(self, provider_name: str, field_name: str) -> str | None:
        """Resolve a credential through the 4-tier priority chain.

        Args:
            provider_name: Provider identifier (e.g., "spotify", "tmdb").
            field_name: Credential field name (e.g., "client_id", "api_key").

        Returns:
            The credential value as a string, or None if not found at any tier.
        """
        # Tier 1: Environment variable (highest priority — user override)
        env_value = self._get_from_env(provider_name, field_name)
        if env_value:
            logger.debug(f"Credential {provider_name}.{field_name} found in environment")
            return env_value

        # Tier 2: Config file (settings.json5 → providers section)
        config_value = self._get_from_config(provider_name, field_name)
        if config_value:
            logger.debug(f"Credential {provider_name}.{field_name} found in config")
            return config_value

        # Tier 3: OS keyring (macOS Keychain, Windows Credential Manager, etc.)
        keyring_value = self._get_from_keyring(provider_name, field_name)
        if keyring_value:
            logger.debug(f"Credential {provider_name}.{field_name} found in keyring")
            return keyring_value

        # Tier 4: Encrypted bundle (app's own keys, if available)
        bundle_value = self._get_from_bundle(provider_name, field_name)
        if bundle_value:
            logger.debug(f"Credential {provider_name}.{field_name} found in encrypted bundle")
            return bundle_value

        # Not found at any tier
        logger.debug(f"Credential {provider_name}.{field_name} not found")
        return None

    def has_credentials(self, provider_name: str) -> bool:
        """Check if all required credentials for a provider are available.

        Providers with no required credentials (e.g., MusicBrainz) always
        return True. Providers with required fields return True only if
        all fields resolve to non-None values.

        Args:
            provider_name: Provider identifier (e.g., "spotify").

        Returns:
            True if all required credentials are present.
        """
        fields = CREDENTIAL_FIELDS.get(provider_name, [])

        # No credentials needed — always available
        if not fields:
            return True

        # Check each required field
        for field_name, _env_var in fields:
            if self.get_credential(provider_name, field_name) is None:
                return False
        return True

    def set_user_credential(self, provider_name: str, field_name: str, value: str) -> bool:
        """Store a user-provided credential in the OS keyring.

        Uses the system's native credential storage (macOS Keychain,
        Windows Credential Manager, Linux SecretService/KDE Wallet).

        Args:
            provider_name: Provider identifier (e.g., "spotify").
            field_name: Credential field name (e.g., "client_id").
            value: The credential value to store.

        Returns:
            True if stored successfully, False on error.
        """
        try:
            import keyring                          # Lazy import — optional dependency
            keyring_key = f"{provider_name}_{field_name}"
            keyring.set_password(self.KEYRING_SERVICE, keyring_key, value)
            logger.info(f"Stored credential {provider_name}.{field_name} in OS keyring")
            return True
        except ImportError:
            logger.warning("keyring package not installed — cannot store in OS keyring")
            return False
        except Exception as e:
            logger.error(f"Failed to store credential in keyring: {e}")
            return False

    def get_env_var_name(self, provider_name: str, field_name: str) -> str:
        """Get the expected environment variable name for a credential.

        Looks up the env var name from CREDENTIAL_FIELDS, or constructs
        one from the provider name and field name.

        Args:
            provider_name: Provider identifier (e.g., "spotify").
            field_name: Credential field name (e.g., "client_id").

        Returns:
            Environment variable name (e.g., "SPOTIFY_CLIENT_ID").
        """
        fields = CREDENTIAL_FIELDS.get(provider_name, [])
        for fname, env_var in fields:
            if fname == field_name:
                return env_var
        # Fallback: construct from provider and field names
        return f"{provider_name.upper()}_{field_name.upper()}"

    def get_required_fields(self, provider_name: str) -> list[tuple[str, str]]:
        """Get the list of required credential fields for a provider.

        Args:
            provider_name: Provider identifier.

        Returns:
            list of (field_name, env_var_name) tuples.
        """
        return CREDENTIAL_FIELDS.get(provider_name, [])

    # ========================================================================
    # Private methods — Individual tier resolution
    # ========================================================================

    def _get_from_env(self, provider_name: str, field_name: str) -> str | None:
        """Tier 1: Look up credential in environment variables.

        Checks the expected env var name (e.g., SPOTIFY_CLIENT_ID).
        Environment variables are loaded from .env by utils/env_loader.py.

        Args:
            provider_name: Provider identifier.
            field_name: Credential field name.

        Returns:
            Value from environment, or None if not set.
        """
        env_var = self.get_env_var_name(provider_name, field_name)
        value = os.environ.get(env_var)
        # Return None for empty strings (treat empty as unset)
        return value if value else None

    def _get_from_config(self, provider_name: str, field_name: str) -> str | None:
        """Tier 2: Look up credential in the config file.

        Checks settings.json5 → providers → {provider_name} → {field_name}.

        Args:
            provider_name: Provider identifier.
            field_name: Credential field name.

        Returns:
            Value from config, or None if not set.
        """
        try:
            if self._config_cache is None:
                from utils.config_loader import load_config
                self._config_cache = load_config() or {}

            providers_config = self._config_cache.get("providers", {})
            provider_config = providers_config.get(provider_name, {})
            value = provider_config.get(field_name)
            return str(value) if value else None
        except Exception:
            return None

    def _get_from_keyring(self, provider_name: str, field_name: str) -> str | None:
        """Tier 3: Look up credential in the OS keyring.

        Uses the keyring library for cross-platform secure storage:
        macOS Keychain, Windows Credential Manager, Linux SecretService.

        Args:
            provider_name: Provider identifier.
            field_name: Credential field name.

        Returns:
            Value from keyring, or None if not stored or keyring unavailable.
        """
        try:
            import keyring                          # Lazy import — optional dependency
            keyring_key = f"{provider_name}_{field_name}"
            value = keyring.get_password(self.KEYRING_SERVICE, keyring_key)
            return value if value else None
        except ImportError:
            # keyring not installed — skip this tier silently
            return None
        except Exception:
            # Keyring access failed — skip gracefully
            return None

    def _get_from_bundle(self, provider_name: str, field_name: str) -> str | None:
        """Tier 4: Look up credential in the encrypted bundle.

        App's own API keys are stored encrypted with a machine-derived
        Fernet key. This prevents casual extraction but is not meant
        to withstand determined reverse engineering.

        The encrypted bundle file is located at:
        metadata/providers/_encrypted_keys.bin

        Args:
            provider_name: Provider identifier.
            field_name: Credential field name.

        Returns:
            Decrypted value from bundle, or None if not available.
        """
        try:
            from cryptography.fernet import Fernet  # Lazy import — optional

            # Locate the encrypted bundle file
            bundle_path = Path(__file__).parent / "_encrypted_keys.bin"
            if not bundle_path.exists():
                return None

            # Derive machine-specific encryption key
            fernet_key = self._derive_machine_key()
            if fernet_key is None:
                return None

            # Decrypt the bundle
            fernet = Fernet(fernet_key)
            encrypted_data = bundle_path.read_bytes()
            import json                             # JSON parsing for decrypted data
            decrypted = json.loads(fernet.decrypt(encrypted_data))

            # Look up the credential
            provider_keys = decrypted.get(provider_name, {})
            value = provider_keys.get(field_name)
            return str(value) if value else None

        except ImportError:
            # cryptography not installed — skip this tier
            return None
        except Exception as e:
            logger.debug(f"Encrypted bundle lookup failed: {e}")
            return None

    def _derive_machine_key(self) -> bytes | None:
        """Derive a Fernet-compatible encryption key from machine identity.

        Combines hostname + MAC address + salt file to create a
        deterministic key unique to this machine. This prevents the
        encrypted bundle from being decrypted on a different machine.

        Returns:
            32-byte URL-safe base64-encoded Fernet key, or None on error.
        """
        try:
            import base64                           # Base64 encoding for Fernet key

            # Machine-specific identifiers
            hostname = platform.node()              # Machine hostname
            mac = str(uuid.getnode())               # MAC address as integer string

            # Salt from a file in user's home directory (created on first run)
            salt_path = Path.home() / ".meedyamanager_salt"
            if salt_path.exists():
                salt = salt_path.read_text().strip()
            else:
                # No salt file — cannot derive key (first run or missing)
                return None

            # Combine and hash to produce exactly 32 bytes for Fernet
            combined = f"{hostname}:{mac}:{salt}"
            key_bytes = hashlib.sha256(combined.encode()).digest()

            # Fernet requires URL-safe base64-encoded 32-byte key
            return base64.urlsafe_b64encode(key_bytes)

        except Exception as e:
            logger.debug(f"Machine key derivation failed: {e}")
            return None
