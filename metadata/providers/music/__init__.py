# ============================================================================
# File: /metadata/providers/music/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Music metadata provider subpackage. Contains provider implementations
# for music-focused services (Apple Music, Spotify, Deezer, Tidal,
# MusicBrainz, YouTube Music, Amazon Music, Pandora, iHeart, Shazam).
#
# All providers in this package use ProviderCategory.MUSIC and are
# auto-discovered by ProviderRegistry.discover() on first access.
# ============================================================================

# Provider modules are imported lazily by ProviderRegistry.discover()
# to avoid loading unused providers and their dependencies at startup.
