# ============================================================================
# File: /metadata/lookup_service.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# The LookupService orchestrates metadata lookups across multiple providers.
# It coordinates provider selection, parallel searching, confidence scoring,
# result ranking, tag writing, and cover art management.
#
# This is the main entry point for metadata lookup operations from both
# the CLI (meedyamanager lookup) and the GUI (Lookup tab).
#
# Architecture:
# - Uses ProviderRegistry for provider discovery and instantiation
# - Uses MatchScorer for confidence-based result ranking
# - Uses CoverArtManager for downloading/saving/embedding artwork
# - Uses TagEditor for writing metadata to files
# - Bridges async providers to sync callers via asyncio.run()
# ============================================================================

import asyncio                                      # Async orchestration for parallel searches
import logging                                      # Standard logging
from pathlib import Path                            # Cross-platform path handling

from metadata.providers import (
    PROVIDER_REGISTRY,                              # Global provider registry singleton
    ProviderCategory,                               # Provider category enum
)
from metadata.providers.base import (
    BaseProvider,                                    # Provider ABC
    ProviderResult,                                  # Result data class
)
from metadata.providers.match_scoring import MatchScorer
from metadata.providers.cover_art import CoverArtManager

logger = logging.getLogger("MeedyaManager.LookupService")


class LookupService:
    """Orchestrates metadata lookups across multiple providers.

    The LookupService is the high-level interface for metadata lookup.
    It handles:
    1. Provider selection (by name, category, or availability)
    2. Parallel searching across multiple providers
    3. Confidence scoring and result ranking
    4. Applying selected results (writing tags + downloading cover art)
    5. Batch processing of multiple files

    Usage:
        service = LookupService()
        results = await service.lookup({"title": "My Song", "artist": "The Band"})
        if results:
            await service.apply_result("song.mp3", results[0])
    """

    def __init__(self):
        """Initialise the lookup service with scorer and cover art manager."""
        self._registry = PROVIDER_REGISTRY          # Provider registry for discovery
        self._scorer = MatchScorer()                # Confidence scoring engine
        self._cover_art_mgr = CoverArtManager()    # Cover art download/save/embed

    async def lookup(self, metadata: dict,
                     providers: list[str] | None = None,
                     category: ProviderCategory | None = None,
                     min_confidence: float = 0.0,
                     max_results_per_provider: int = 5) -> list[ProviderResult]:
        """Search multiple providers and return ranked results.

        Searches are run in parallel (async) across all selected providers.
        Results are scored against the query metadata and sorted by confidence.

        Args:
            metadata: Current file metadata (title, artist, album, isrc, etc.)
            providers: Specific provider names to search (None = all available).
            category: Filter by category (None = all categories).
            min_confidence: Minimum match confidence to include (0.0-1.0).
            max_results_per_provider: Maximum results from each provider.

        Returns:
            list[ProviderResult]: All results, scored and sorted by confidence.
        """
        # Determine which providers to search
        selected_providers = self._select_providers(providers, category)

        if not selected_providers:
            logger.warning("No available providers for lookup")
            return []

        provider_names = [p.provider_name for p in selected_providers]
        logger.info(f"Searching {len(selected_providers)} providers: {provider_names}")

        # Run all provider searches in parallel
        all_results: list[ProviderResult] = []
        tasks = []
        for provider in selected_providers:
            task = self._search_provider(provider, metadata, max_results_per_provider)
            tasks.append(task)

        # Gather results from all providers (continue even if some fail)
        provider_results = await asyncio.gather(*tasks, return_exceptions=True)

        for i, result in enumerate(provider_results):
            if isinstance(result, Exception):
                logger.error(
                    f"Provider {selected_providers[i].provider_name} failed: {result}"
                )
            elif isinstance(result, list):
                all_results.extend(result)

        # Score and rank all results
        if all_results:
            all_results = self._scorer.rank_results(metadata, all_results)

        # Filter by minimum confidence
        if min_confidence > 0.0:
            all_results = [r for r in all_results if r.confidence >= min_confidence]

        logger.info(
            f"Lookup complete: {len(all_results)} results "
            f"(min confidence: {min_confidence})"
        )
        return all_results

    def lookup_sync(self, metadata: dict, **kwargs) -> list[ProviderResult]:
        """Synchronous wrapper for lookup() — for CLI and non-async callers.

        Creates a new event loop, runs the async lookup, and returns results.

        Args:
            metadata: Current file metadata dict.
            **kwargs: Additional arguments passed to lookup().

        Returns:
            list[ProviderResult]: Scored and sorted results.
        """
        try:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                return loop.run_until_complete(self.lookup(metadata, **kwargs))
            finally:
                loop.close()
        except Exception as e:
            logger.error(f"Sync lookup failed: {e}")
            return []

    async def apply_result(self, filepath: str, result: ProviderResult,
                           write_tags: bool = True,
                           download_art: bool = True,
                           dry_run: bool = False) -> dict:
        """Apply a selected provider result to a media file.

        Writes standard tags (title, artist, album, etc.) and custom tags
        (provider ID, URL, extra metadata) to the file via TagEditor.
        Downloads and saves cover art if available.

        Args:
            filepath: Path to the media file to update.
            result: The ProviderResult to apply.
            write_tags: Whether to write metadata tags to the file.
            download_art: Whether to download and save cover art.
            dry_run: If True, compute changes but don't write anything.

        Returns:
            dict with keys:
            - "tags_written": dict of {key: value} tags written
            - "cover_art_saved": dict of {type: path} cover art files saved
            - "dry_run": whether this was a dry run
        """
        changes = {
            "tags_written": {},
            "cover_art_saved": {},
            "dry_run": dry_run,
        }

        # Collect all tags to write
        all_tags = result.get_all_tags()

        if write_tags and all_tags:
            if dry_run:
                # Preview only — don't actually write
                changes["tags_written"] = all_tags
                logger.info(f"[DRY RUN] Would write {len(all_tags)} tags to {filepath}")
            else:
                try:
                    from metadata.editor import TagEditor
                    editor = TagEditor()
                    written = editor.write_tags(filepath, all_tags)
                    changes["tags_written"] = all_tags
                    logger.info(f"Wrote {len(all_tags)} tags to {filepath}")
                except Exception as e:
                    logger.error(f"Failed to write tags to {filepath}: {e}")

        # Download and save cover art
        if download_art and result.cover_art:
            if dry_run:
                # Preview only — list what would be downloaded
                for asset in result.cover_art:
                    changes["cover_art_saved"][asset.asset_type.value] = (
                        f"[DRY RUN] Would download {asset.url[:60]}..."
                    )
                logger.info(
                    f"[DRY RUN] Would download {len(result.cover_art)} "
                    f"cover art assets"
                )
            else:
                saved = await self._cover_art_mgr.process_cover_art(
                    filepath, result.cover_art
                )
                changes["cover_art_saved"] = saved

        return changes

    def apply_result_sync(self, filepath: str, result: ProviderResult,
                          **kwargs) -> dict:
        """Synchronous wrapper for apply_result() — for CLI and non-async callers.

        Args:
            filepath: Path to the media file.
            result: The ProviderResult to apply.
            **kwargs: Additional arguments passed to apply_result().

        Returns:
            dict of changes made.
        """
        try:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                return loop.run_until_complete(
                    self.apply_result(filepath, result, **kwargs)
                )
            finally:
                loop.close()
        except Exception as e:
            logger.error(f"Sync apply_result failed: {e}")
            return {"tags_written": {}, "cover_art_saved": {}, "error": str(e)}

    async def batch_lookup(self, filepaths: list[str],
                           providers: list[str] | None = None,
                           auto_apply: bool = False,
                           min_confidence: float = 0.8) -> list[dict]:
        """Batch lookup for multiple files with optional auto-apply.

        Processes each file sequentially (to respect rate limits) but
        searches providers in parallel for each file.

        Args:
            filepaths: List of media file paths to process.
            providers: Specific provider names (None = all available).
            auto_apply: If True, automatically apply the best match
                       (if confidence >= min_confidence).
            min_confidence: Minimum confidence for auto-apply.

        Returns:
            list[dict]: One result dict per file with keys:
            - "filepath": file path
            - "results": list of ProviderResult
            - "applied": the result that was applied (or None)
            - "changes": dict of changes made (or None)
        """
        batch_results = []

        for i, filepath in enumerate(filepaths):
            logger.info(f"Batch lookup [{i + 1}/{len(filepaths)}]: {Path(filepath).name}")

            # Extract metadata from the file
            try:
                from core.metadata_extractor import extract_metadata
                metadata = extract_metadata(filepath)
            except Exception as e:
                logger.error(f"Failed to extract metadata from {filepath}: {e}")
                batch_results.append({
                    "filepath": filepath,
                    "results": [],
                    "applied": None,
                    "changes": None,
                    "error": str(e),
                })
                continue

            # Search providers
            results = await self.lookup(
                metadata, providers=providers, min_confidence=min_confidence
            )

            entry = {
                "filepath": filepath,
                "results": results,
                "applied": None,
                "changes": None,
            }

            # Auto-apply best match if requested
            if auto_apply and results and results[0].confidence >= min_confidence:
                best = results[0]
                changes = await self.apply_result(filepath, best)
                entry["applied"] = best
                entry["changes"] = changes
                logger.info(
                    f"Auto-applied {best.provider_name} result "
                    f"(confidence: {best.confidence:.2f}) to {Path(filepath).name}"
                )

            batch_results.append(entry)

        return batch_results

    def get_available_providers(self) -> list[dict]:
        """List all providers with their availability status.

        Returns a list of status dicts suitable for CLI --providers-list
        and GUI provider status display.

        Returns:
            list[dict]: Each dict has keys: name, category, requires_auth,
                       available, message.
        """
        all_providers = self._registry.get_all()
        return [p.get_status_info() for p in all_providers]

    # ========================================================================
    # Private helper methods
    # ========================================================================

    def _select_providers(self, names: list[str] | None,
                          category: ProviderCategory | None) -> list[BaseProvider]:
        """Select providers based on names and/or category filter.

        If names is provided, return those specific providers (if available).
        If category is provided, filter by category.
        If neither, return all available providers.

        Args:
            names: Specific provider names to use (None = all).
            category: Category filter (None = all categories).

        Returns:
            list[BaseProvider]: Selected providers that are available.
        """
        if names:
            # User specified specific providers
            selected = []
            for name in names:
                provider = self._registry.get_provider(name)
                if provider is not None and provider.is_available():
                    selected.append(provider)
                elif provider is not None:
                    logger.warning(f"Provider {name} is not available")
                else:
                    logger.warning(f"Provider {name} not found")
            return selected

        if category:
            # Filter by category
            return [
                p for p in self._registry.get_by_category(category)
                if p.is_available()
            ]

        # All available providers
        return self._registry.get_available()

    async def _search_provider(self, provider: BaseProvider, metadata: dict,
                                max_results: int) -> list[ProviderResult]:
        """Search a single provider with error handling.

        Args:
            provider: The provider to search.
            metadata: File metadata to search by.
            max_results: Maximum number of results to return.

        Returns:
            list[ProviderResult]: Results from this provider.
        """
        try:
            results = await provider.search(metadata)
            # Limit results per provider
            if len(results) > max_results:
                results = results[:max_results]
            logger.debug(
                f"Provider {provider.provider_name}: {len(results)} results"
            )
            return results
        except Exception as e:
            logger.error(f"Provider {provider.provider_name} search failed: {e}")
            return []
