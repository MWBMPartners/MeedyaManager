# ============================================================================
# File: /tests/test_rate_limiter.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the token bucket rate limiter:
# - Token bucket algorithm (refill, consume, capacity)
# - Synchronous acquire and wait behaviour
# - Async acquire behaviour
# - Exponential backoff for 429 responses
# - Pre-configured rate limits for all 19 providers
# - get_rate_limiter() function
# ============================================================================

import asyncio                                             # For running async tests
import time                                                # Timing verification
import pytest                                              # Test framework

from metadata.providers.rate_limiter import (
    RateLimiter,                                           # Main class under test
    RATE_LIMITS,                                           # Pre-configured limiters
    get_rate_limiter,                                      # Limiter lookup function
)


# =============================================================================
# Token Bucket Algorithm Tests
# =============================================================================

class TestTokenBucket:
    """Tests for the core token bucket algorithm."""

    def test_initial_tokens(self):
        """A new limiter should start with full token bucket."""
        limiter = RateLimiter(10, 10.0)
        assert limiter.tokens_available == pytest.approx(10.0, abs=0.5)

    def test_consume_token(self):
        """acquire_sync() should consume one token."""
        limiter = RateLimiter(10, 10.0)
        limiter.acquire_sync()
        assert limiter.tokens_available < 10.0

    def test_multiple_consumes(self):
        """Multiple acquire_sync() calls should consume multiple tokens."""
        limiter = RateLimiter(5, 10.0)
        for _ in range(3):
            limiter.acquire_sync()
        # Should have approximately 2 tokens left (minus refill time)
        assert limiter.tokens_available < 3.0

    def test_tokens_capped_at_max(self):
        """Tokens should not exceed the maximum capacity."""
        limiter = RateLimiter(5, 10.0)
        # Force a long elapsed time for refill
        limiter._last_refill = time.monotonic() - 100      # 100 seconds ago
        limiter._refill()
        assert limiter._tokens <= limiter._max_tokens

    def test_refill_rate(self):
        """Tokens should refill at the correct rate."""
        limiter = RateLimiter(10, 10.0)                    # 1 token per second
        # Consume all tokens
        for _ in range(10):
            limiter._tokens -= 1.0
        assert limiter._tokens == pytest.approx(0.0, abs=0.1)

        # Simulate 5 seconds of refill
        limiter._last_refill = time.monotonic() - 5.0
        limiter._refill()
        assert limiter._tokens == pytest.approx(5.0, abs=0.5)


# =============================================================================
# Synchronous Acquire Tests
# =============================================================================

class TestAcquireSync:
    """Tests for the synchronous acquire_sync() method."""

    def test_acquire_no_wait_when_tokens_available(self):
        """acquire_sync() should not block when tokens are available."""
        limiter = RateLimiter(10, 10.0)
        start = time.monotonic()
        limiter.acquire_sync()
        elapsed = time.monotonic() - start
        assert elapsed < 0.1                               # Should be near-instant

    def test_acquire_blocks_when_empty(self):
        """acquire_sync() should block briefly when tokens are exhausted."""
        # Very fast refill rate to avoid long waits
        limiter = RateLimiter(1, 0.2)                      # 1 token per 0.2 seconds
        limiter.acquire_sync()                              # Consume the only token
        start = time.monotonic()
        limiter.acquire_sync()                              # Should wait for refill
        elapsed = time.monotonic() - start
        assert elapsed >= 0.05                              # Should have waited


# =============================================================================
# Async Acquire Tests
# =============================================================================

class TestAcquireAsync:
    """Tests for the async acquire() method."""

    def test_async_acquire_available(self):
        """async acquire() should succeed when tokens are available."""
        limiter = RateLimiter(10, 10.0)

        async def run():
            await limiter.acquire()
            return True

        result = asyncio.run(run())
        assert result is True

    def test_async_acquire_multiple(self):
        """Multiple async acquire() calls should work sequentially."""
        limiter = RateLimiter(5, 10.0)

        async def run():
            for _ in range(3):
                await limiter.acquire()
            return True

        result = asyncio.run(run())
        assert result is True


# =============================================================================
# Rate Limit Backoff Tests
# =============================================================================

class TestBackoff:
    """Tests for exponential backoff on 429 responses."""

    def test_handle_rate_limit_sync_with_retry_after(self):
        """handle_rate_limit_sync() should respect Retry-After value."""
        limiter = RateLimiter(10, 10.0)
        start = time.monotonic()
        waited = limiter.handle_rate_limit_sync(retry_after=0.1, attempt=0)
        elapsed = time.monotonic() - start
        assert waited == pytest.approx(0.1, abs=0.05)
        assert elapsed >= 0.08

    def test_handle_rate_limit_sync_drains_tokens(self):
        """handle_rate_limit_sync() should drain all tokens."""
        limiter = RateLimiter(10, 10.0)
        limiter.handle_rate_limit_sync(retry_after=0.05, attempt=0)
        assert limiter._tokens == 0.0

    def test_handle_rate_limit_async_with_retry_after(self):
        """async handle_rate_limit() should respect Retry-After value."""
        limiter = RateLimiter(10, 10.0)

        async def run():
            waited = await limiter.handle_rate_limit(retry_after=0.1, attempt=0)
            return waited

        waited = asyncio.run(run())
        assert waited == pytest.approx(0.1, abs=0.05)

    def test_backoff_increases_with_attempts(self):
        """Backoff delay should increase with higher attempt numbers."""
        limiter = RateLimiter(10, 10.0, backoff_factor=2.0)
        # Attempt 0: base = 2^0 = 1 + jitter
        # Attempt 2: base = 2^2 = 4 + jitter
        # We just verify attempt 2 has longer wait than attempt 0
        wait_0 = limiter.handle_rate_limit_sync(retry_after=None, attempt=0)
        wait_2 = limiter.handle_rate_limit_sync(retry_after=None, attempt=2)
        # attempt=2 should be longer (4x base vs 1x base)
        # Note: jitter makes this non-deterministic, but the base difference is 3s
        assert wait_2 > wait_0


# =============================================================================
# Pre-configured Rate Limits Tests
# =============================================================================

class TestRateLimits:
    """Tests for the pre-configured RATE_LIMITS dictionary."""

    def test_all_19_providers_configured(self):
        """All 19 providers should have pre-configured rate limiters."""
        expected_providers = {
            "apple_music", "spotify", "musicbrainz", "deezer",
            "youtube_music", "amazon_music", "pandora", "tidal",
            "shazam", "iheart", "tmdb", "tvdb", "imdb",
            "apple_tv", "itunes_store", "apple_podcasts",
            "isrc", "eidr", "iswc",
        }
        actual_providers = set(RATE_LIMITS.keys())
        assert expected_providers == actual_providers

    def test_musicbrainz_strict_limit(self):
        """MusicBrainz should have a strict 1 req/sec rate limit."""
        limiter = RATE_LIMITS["musicbrainz"]
        assert limiter.requests_per_window == 1
        assert limiter.window_seconds == 1.0

    def test_spotify_limit(self):
        """Spotify should have a 40/10s rate limit."""
        limiter = RATE_LIMITS["spotify"]
        assert limiter.requests_per_window == 40
        assert limiter.window_seconds == 10.0

    def test_deezer_limit(self):
        """Deezer should have a 50/5s rate limit."""
        limiter = RATE_LIMITS["deezer"]
        assert limiter.requests_per_window == 50
        assert limiter.window_seconds == 5.0

    def test_all_limiters_are_instances(self):
        """All values in RATE_LIMITS should be RateLimiter instances."""
        for name, limiter in RATE_LIMITS.items():
            assert isinstance(limiter, RateLimiter), f"{name} is not a RateLimiter"


# =============================================================================
# get_rate_limiter() Tests
# =============================================================================

class TestGetRateLimiter:
    """Tests for the get_rate_limiter() function."""

    def test_known_provider(self):
        """Should return the pre-configured limiter for known providers."""
        limiter = get_rate_limiter("spotify")
        assert limiter is RATE_LIMITS["spotify"]

    def test_unknown_provider_returns_default(self):
        """Should return a default limiter for unknown providers."""
        limiter = get_rate_limiter("nonexistent_provider")
        assert isinstance(limiter, RateLimiter)
        assert limiter.requests_per_window == 10           # Default: 10 req/10s
        assert limiter.window_seconds == 10.0
