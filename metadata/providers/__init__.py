# ============================================================================
# File: /metadata/providers/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Provider registry and discovery system for metadata lookup providers.
# Manages registration, retrieval, and auto-discovery of all metadata
# providers across music, video, podcast, and identifier categories.
#
# The ProviderRegistry is a singleton that holds references to all
# available provider classes. Providers register themselves via the
# @register_provider decorator or by calling registry.register().
#
# Auto-discovery scans the music/, video/, podcasts/, and identifiers/
# subpackages to import all provider modules on first access.
# ============================================================================

import importlib                                    # Dynamic module loading for auto-discovery
import logging                                      # Standard logging for discovery events
from enum import Enum                               # Enum for provider categories


# ============================================================================
# Logger — Used for provider discovery and registration events
# ============================================================================
logger = logging.getLogger("MeedyaManager.ProviderRegistry")


# ============================================================================
# ProviderCategory — Classifies providers by their content domain.
# Used for filtering providers in CLI (--category) and GUI (tab grouping).
# ============================================================================
class ProviderCategory(Enum):
    """Categories that group metadata providers by content domain."""
    MUSIC = "music"                                 # Music streaming/catalog providers
    VIDEO = "video"                                 # Movie and TV show providers
    PODCAST = "podcast"                             # Podcast catalog providers
    IDENTIFIER = "identifier"                       # Industry identifier registries (ISRC, EIDR, ISWC)


# ============================================================================
# ProviderRegistry — Central registry for all metadata providers.
#
# Providers are registered by class reference (not instance). The registry
# creates instances lazily when get_provider() is called, passing the
# CredentialManager for authentication.
# ============================================================================
class ProviderRegistry:
    """Central registry that manages all metadata provider classes.

    Providers register their class via register(), and instances are
    created on demand when get_provider() is called. This allows the
    registry to exist before any provider credentials are configured.
    """

    def __init__(self):
        """Initialise an empty provider registry."""
        self._providers: dict[str, type] = {}       # name → provider class
        self._instances: dict[str, object] = {}     # name → provider instance (lazy)
        self._discovered: bool = False              # Whether auto-discovery has run

    def register(self, provider_class: type) -> None:
        """Register a provider class by its name property.

        Args:
            provider_class: A subclass of BaseProvider to register.
        """
        # Instantiate temporarily to read the name property
        name = provider_class.provider_name         # Class-level attribute, not instance
        self._providers[name] = provider_class
        logger.debug(f"Registered provider: {name}")

    def get_provider(self, name: str):
        """Get a provider instance by name, creating it if needed.

        Args:
            name: The provider's registered name (e.g., "spotify", "apple_music").

        Returns:
            BaseProvider instance, or None if not found.
        """
        # Ensure discovery has run before looking up
        if not self._discovered:
            self.discover()

        # Return cached instance if available
        if name in self._instances:
            return self._instances[name]

        # Create new instance from registered class
        provider_class = self._providers.get(name)
        if provider_class is None:
            logger.warning(f"Provider not found: {name}")
            return None

        try:
            instance = provider_class()             # Providers use CredentialManager internally
            self._instances[name] = instance
            return instance
        except Exception as e:
            logger.error(f"Failed to instantiate provider {name}: {e}")
            return None

    def get_all(self) -> list:
        """Get instances of all registered providers.

        Returns:
            list[BaseProvider]: All provider instances.
        """
        if not self._discovered:
            self.discover()

        result = []
        for name in self._providers:
            provider = self.get_provider(name)
            if provider is not None:
                result.append(provider)
        return result

    def get_by_category(self, category: ProviderCategory) -> list:
        """Get all provider instances matching a specific category.

        Args:
            category: The ProviderCategory to filter by.

        Returns:
            list[BaseProvider]: Providers in the specified category.
        """
        return [
            p for p in self.get_all()
            if p.category == category
        ]

    def get_available(self) -> list:
        """Get all providers that have valid credentials and are operational.

        Returns:
            list[BaseProvider]: Only providers where is_available() returns True.
        """
        return [p for p in self.get_all() if p.is_available()]

    def get_registered_names(self) -> list[str]:
        """Get a sorted list of all registered provider names.

        Returns:
            list[str]: Provider names (e.g., ["apple_music", "deezer", "spotify"]).
        """
        if not self._discovered:
            self.discover()
        return sorted(self._providers.keys())

    def discover(self) -> None:
        """Auto-discover providers by importing all subpackage modules.

        Scans the music/, video/, podcasts/, and identifiers/ subpackages
        and imports each module. Provider modules register themselves
        during import via the register_provider() function.
        """
        if self._discovered:
            return

        # List of subpackages to scan for provider modules
        subpackages = [
            "metadata.providers.music",
            "metadata.providers.video",
            "metadata.providers.podcasts",
            "metadata.providers.identifiers",
        ]

        for package_name in subpackages:
            try:
                package = importlib.import_module(package_name)
                # Each subpackage __init__.py imports its provider modules
                logger.debug(f"Discovered providers from {package_name}")
            except ImportError as e:
                # Subpackage may not exist yet during development
                logger.debug(f"Could not import {package_name}: {e}")

        self._discovered = True
        logger.info(f"Provider discovery complete: {len(self._providers)} providers registered")


# ============================================================================
# Global Registry Singleton — Single instance shared across the application
# ============================================================================
PROVIDER_REGISTRY = ProviderRegistry()


def register_provider(provider_class: type) -> type:
    """Decorator to register a provider class with the global registry.

    Usage:
        @register_provider
        class SpotifyProvider(BaseProvider):
            provider_name = "spotify"
            ...

    Args:
        provider_class: The provider class to register.

    Returns:
        The provider class (unchanged), allowing use as a decorator.
    """
    PROVIDER_REGISTRY.register(provider_class)
    return provider_class
