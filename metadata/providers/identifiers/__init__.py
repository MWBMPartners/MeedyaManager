# ============================================================================
# File: /metadata/providers/identifiers/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Identifier lookup provider subpackage. Contains provider implementations
# for identifier registries and cross-reference services (ISRC, EIDR, ISWC).
#
# All providers in this package use ProviderCategory.IDENTIFIER and are
# auto-discovered by ProviderRegistry.discover() on first access.
# ============================================================================

# Provider modules are imported lazily by ProviderRegistry.discover()
# to avoid loading unused providers and their dependencies at startup.
