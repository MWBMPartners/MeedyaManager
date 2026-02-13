# ============================================================================
# File: /metadata/providers/video/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Video metadata provider subpackage. Contains provider implementations
# for TV and film services (TMDB, TheTVDB, IMDb, Apple TV, iTunes Store).
#
# All providers in this package use ProviderCategory.VIDEO and are
# auto-discovered by ProviderRegistry.discover() on first access.
# ============================================================================

# Provider modules are imported lazily by ProviderRegistry.discover()
# to avoid loading unused providers and their dependencies at startup.
