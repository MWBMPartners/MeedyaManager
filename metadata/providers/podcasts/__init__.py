# ============================================================================
# File: /metadata/providers/podcasts/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Podcast metadata provider subpackage. Contains provider implementations
# for podcast-focused services (Apple Podcasts).
#
# All providers in this package use ProviderCategory.PODCAST and are
# auto-discovered by ProviderRegistry.discover() on first access.
# ============================================================================

# Provider modules are imported lazily by ProviderRegistry.discover()
# to avoid loading unused providers and their dependencies at startup.
