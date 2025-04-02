# Redis Rate Limiter

A thread-safe rate limiter implementation in Rust using Redis as the backend storage. This library provides a simple and efficient way to implement rate limiting in your applications.

## Features

- Thread-safe rate limiting using Redis
- Configurable request limits and time windows
- Support for multiple identifiers (e.g., per user, IP, endpoint)
- Atomic operations using Redis Lua scripts
- Built-in methods to check remaining requests and time windows
- Comprehensive test suite

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
redis_rate_limiter = "0.1.0"
```

## Usage

```rust
use redis_rate_limiter::{RateLimiter, RateLimiterError};
use std::time::Duration;

fn main() -> Result<(), RateLimiterError> {
    // Create a new rate limiter instance
    let limiter = RateLimiter::new(
        "redis://127.0.0.1:6379",  // Redis URL
        "my_app",                  // Key prefix
        100,                       // Max requests
        Duration::from_secs(60),   // Time window
    )?;

    // Check if a request should be allowed
    match limiter.check("user_123") {
        Ok(_) => println!("Request allowed"),
        Err(RateLimiterError::RateLimitExceeded) => println!("Rate limit exceeded"),
        Err(e) => println!("Error: {}", e),
    }

    // Get remaining requests
    let remaining = limiter.get_remaining("user_123")?;
    println!("Remaining requests: {}", remaining);

    // Get time until rate limit resets
    let time_remaining = limiter.get_time_remaining("user_123")?;
    println!("Time remaining: {} seconds", time_remaining);

    Ok(())
}
```

## API

### RateLimiter

The main struct that handles rate limiting operations.

#### Methods

- `new(redis_url: &str, key_prefix: &str, max_requests: u64, window: Duration) -> Result<Self, RateLimiterError>`
  - Creates a new rate limiter instance
  - `redis_url`: URL of the Redis server
  - `key_prefix`: Prefix for Redis keys
  - `max_requests`: Maximum number of requests allowed in the time window
  - `window`: Duration of the time window

- `check(identifier: &str) -> Result<(), RateLimiterError>`
  - Checks if a request should be allowed
  - Returns `Ok(())` if the request is allowed
  - Returns `Err(RateLimiterError::RateLimitExceeded)` if the rate limit is exceeded

- `get_remaining(identifier: &str) -> Result<u64, RateLimiterError>`
  - Returns the number of remaining requests for the given identifier

- `get_time_remaining(identifier: &str) -> Result<i64, RateLimiterError>`
  - Returns the time remaining until the rate limit resets (in seconds)
  - Returns -1 if the key has expired or doesn't exist

### RateLimiterError

Error type for rate limiter operations.

```rust
pub enum RateLimiterError {
    Redis(redis::RedisError),
    RateLimitExceeded,
}
```

## Requirements

- Redis server (version 2.6 or later)
- Rust 1.70 or later

## Testing

The library includes a comprehensive test suite. To run the tests:

```bash
cargo test
```

Note: Tests require a running Redis server at `redis://127.0.0.1:6379`. You can use Docker to run Redis:

```bash
docker run -d -p 6379:6379 redis
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 