# ============================================================================
# File: /metadata/providers/rate_limiter.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Per-provider rate limiting system using a token bucket algorithm.
# Each metadata provider has its own rate limiter configured to respect
# that provider's API limits (e.g., MusicBrainz: 1 req/sec, Spotify:
# ~4000/10min, TMDB: 40/10s).
#
# Features:
# - Token bucket algorithm with configurable window and capacity
# - Exponential backoff for HTTP 429 (Too Many Requests) responses
# - Both async (await acquire()) and sync (acquire_sync()) interfaces
# - Pre-configured rate limits for all supported providers
# ============================================================================

import asyncio                                      # Async lock and sleep for acquire()
import time                                         # Time tracking for token refill
import random                                       # Jitter for backoff to avoid thundering herd
import logging                                      # Standard logging

logger = logging.getLogger("MeedyaManager.RateLimiter")


class RateLimiter:
    """Token bucket rate limiter with exponential backoff support.

    The token bucket algorithm allows bursts up to the bucket capacity,
    then throttles to the refill rate. This matches most API rate limit
    patterns (e.g., "40 requests per 10 seconds").

    Usage:
        limiter = RateLimiter(requests_per_window=40, window_seconds=10.0)
        await limiter.acquire()  # Waits if bucket is empty
        response = await make_api_call()
        if response.status_code == 429:
            await limiter.handle_rate_limit(retry_after=60)
    """

    def __init__(self, requests_per_window: int, window_seconds: float,
                 max_retries: int = 3, backoff_factor: float = 2.0):
        """Initialise a rate limiter with token bucket parameters.

        Args:
            requests_per_window: Maximum requests allowed per window.
            window_seconds: Duration of the rate limit window in seconds.
            max_retries: Maximum number of retries on rate limit errors.
            backoff_factor: Multiplier for exponential backoff between retries.
        """
        self.requests_per_window = requests_per_window  # Bucket capacity
        self.window_seconds = window_seconds        # Time window for the limit
        self.max_retries = max_retries              # Max retries on 429 errors
        self.backoff_factor = backoff_factor         # Exponential backoff multiplier

        # Token bucket state
        self._tokens: float = float(requests_per_window)  # Current tokens available
        self._max_tokens: float = float(requests_per_window)  # Maximum bucket capacity
        self._refill_rate: float = requests_per_window / window_seconds  # Tokens per second
        self._last_refill: float = time.monotonic()  # Timestamp of last token refill

        # Async lock to prevent concurrent token consumption
        self._lock: asyncio.Lock | None = None      # Created lazily on first async use

    async def acquire(self) -> None:
        """Wait until a request token is available (async version).

        Blocks if the token bucket is empty, waiting for tokens to refill.
        Uses an asyncio.Lock to prevent race conditions in concurrent use.
        """
        # Create lock lazily (must be created in an event loop context)
        if self._lock is None:
            self._lock = asyncio.Lock()

        async with self._lock:
            # Refill tokens based on elapsed time
            self._refill()

            if self._tokens >= 1.0:
                # Token available — consume one and proceed
                self._tokens -= 1.0
            else:
                # No tokens — calculate wait time until next token
                wait_time = (1.0 - self._tokens) / self._refill_rate
                logger.debug(f"Rate limit: waiting {wait_time:.2f}s for token")
                await asyncio.sleep(wait_time)
                self._refill()
                self._tokens = max(0.0, self._tokens - 1.0)

    def acquire_sync(self) -> None:
        """Wait until a request token is available (synchronous version).

        Blocks the current thread if the token bucket is empty.
        Use this in non-async contexts (CLI commands, QThread workers).
        """
        self._refill()

        if self._tokens >= 1.0:
            # Token available — consume one and proceed
            self._tokens -= 1.0
        else:
            # No tokens — sleep until next token is available
            wait_time = (1.0 - self._tokens) / self._refill_rate
            logger.debug(f"Rate limit: sleeping {wait_time:.2f}s for token")
            time.sleep(wait_time)
            self._refill()
            self._tokens = max(0.0, self._tokens - 1.0)

    async def handle_rate_limit(self, retry_after: float | None = None,
                                 attempt: int = 0) -> float:
        """Handle a 429 (Too Many Requests) response with exponential backoff.

        If the API provides a Retry-After header, use that value.
        Otherwise, calculate exponential backoff with jitter.

        Args:
            retry_after: Seconds to wait (from Retry-After header), or None.
            attempt: Current retry attempt number (0-based).

        Returns:
            The actual time waited in seconds.
        """
        if retry_after is not None and retry_after > 0:
            # API specified retry delay — respect it
            wait_time = retry_after
            logger.warning(f"Rate limited — waiting {wait_time:.1f}s (Retry-After)")
        else:
            # Calculate exponential backoff with jitter
            base_wait = self.backoff_factor ** attempt
            jitter = random.uniform(0, 1.0)         # Add jitter to avoid thundering herd
            wait_time = base_wait + jitter
            logger.warning(f"Rate limited — backoff {wait_time:.1f}s (attempt {attempt + 1})")

        # Drain tokens to prevent immediate re-request
        self._tokens = 0.0

        await asyncio.sleep(wait_time)
        return wait_time

    def handle_rate_limit_sync(self, retry_after: float | None = None,
                                attempt: int = 0) -> float:
        """Synchronous version of handle_rate_limit().

        Args:
            retry_after: Seconds to wait (from Retry-After header), or None.
            attempt: Current retry attempt number (0-based).

        Returns:
            The actual time waited in seconds.
        """
        if retry_after is not None and retry_after > 0:
            wait_time = retry_after
            logger.warning(f"Rate limited — waiting {wait_time:.1f}s (Retry-After)")
        else:
            base_wait = self.backoff_factor ** attempt
            jitter = random.uniform(0, 1.0)
            wait_time = base_wait + jitter
            logger.warning(f"Rate limited — backoff {wait_time:.1f}s (attempt {attempt + 1})")

        self._tokens = 0.0
        time.sleep(wait_time)
        return wait_time

    def _refill(self) -> None:
        """Refill the token bucket based on elapsed time.

        Adds tokens proportional to the time elapsed since the last
        refill, capped at the maximum bucket capacity.
        """
        now = time.monotonic()
        elapsed = now - self._last_refill
        self._last_refill = now

        # Add tokens based on elapsed time and refill rate
        new_tokens = elapsed * self._refill_rate
        self._tokens = min(self._max_tokens, self._tokens + new_tokens)

    @property
    def tokens_available(self) -> float:
        """Get the current number of available tokens (for diagnostics).

        Returns:
            float: Current token count (may be fractional).
        """
        self._refill()
        return self._tokens


# ============================================================================
# Pre-configured Rate Limits — One per provider, based on API documentation.
#
# Providers with undocumented rate limits use conservative defaults.
# Users can override these via config if needed in future versions.
# ============================================================================
RATE_LIMITS: dict[str, RateLimiter] = {
    # Music providers
    "apple_music": RateLimiter(20, 10.0),           # Conservative (Apple undocumented)
    "spotify": RateLimiter(40, 10.0),               # ~4000/10min → 40/10s window
    "musicbrainz": RateLimiter(1, 1.0),             # 1 req/sec strictly enforced
    "deezer": RateLimiter(50, 5.0),                 # 50 requests per 5 seconds
    "youtube_music": RateLimiter(5, 10.0),           # Conservative (unofficial API)
    "amazon_music": RateLimiter(5, 10.0),           # Conservative (closed beta)
    "pandora": RateLimiter(5, 10.0),                # Conservative (no public API)
    "tidal": RateLimiter(10, 10.0),                 # Conservative (undocumented)
    "shazam": RateLimiter(5, 10.0),                 # Conservative (reverse-engineered)
    "iheart": RateLimiter(5, 10.0),                 # Conservative (undocumented)

    # Video providers
    "tmdb": RateLimiter(40, 10.0),                  # 40 requests per 10 seconds
    "tvdb": RateLimiter(1, 1.0),                    # 1 request per second
    "imdb": RateLimiter(5, 10.0),                   # Conservative (scraping)
    "apple_tv": RateLimiter(20, 60.0),              # Apple Search API rate
    "itunes_store": RateLimiter(20, 60.0),          # Apple Search API rate

    # Podcast providers
    "apple_podcasts": RateLimiter(20, 60.0),        # Apple Search API rate

    # Identifier providers
    "isrc": RateLimiter(10, 10.0),                  # Federated (limited by sub-providers)
    "eidr": RateLimiter(10, 10.0),                  # Conservative (proprietary service)
    "iswc": RateLimiter(1, 1.0),                    # Limited by MusicBrainz (1/sec)
}


def get_rate_limiter(provider_name: str) -> RateLimiter:
    """Get the pre-configured rate limiter for a provider.

    Returns a provider-specific rate limiter if configured, or a
    generous default for unknown providers.

    Args:
        provider_name: The provider's registered name.

    Returns:
        RateLimiter instance configured for the provider's API limits.
    """
    if provider_name in RATE_LIMITS:
        return RATE_LIMITS[provider_name]

    # Default rate limiter for unknown providers (generous)
    logger.debug(f"No rate limiter configured for {provider_name}, using default")
    return RateLimiter(10, 10.0)
