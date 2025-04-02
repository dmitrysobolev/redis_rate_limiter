use std::time::Duration;
use redis::Commands;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RateLimiterError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

pub struct RateLimiter {
    redis_client: redis::Client,
    key_prefix: String,
    max_requests: u64,
    window: Duration,
}

impl RateLimiter {
    /// Creates a new RateLimiter instance.
    pub fn new(
        redis_url: &str,
        key_prefix: &str,
        max_requests: u64,
        window: Duration,
    ) -> Result<Self, RateLimiterError> {
        let client = redis::Client::open(redis_url)?;
        Ok(RateLimiter {
            redis_client: client,
            key_prefix: key_prefix.to_string(),
            max_requests,
            window,
        })
    }

    fn get_redis_key(&self, identifier: &str) -> String {
        format!("{}:{}", self.key_prefix, identifier)
    }

    pub fn check(&self, identifier: &str) -> Result<(), RateLimiterError> {
        let key = self.get_redis_key(identifier);
        let mut conn = self.redis_client.get_connection()?;
        let window_seconds = self.window.as_secs() as usize;

        let script = redis::Script::new(r#"
            local key = KEYS[1]
            local limit = tonumber(ARGV[1])
            local expiry = tonumber(ARGV[2])
            local current = redis.call("INCR", key)
            if current > limit then
                return 0
            else
                redis.call("EXPIRE", key, expiry)
                return 1
            end
        "#);

        let result: Result<u64, redis::RedisError> = script
            .key(&key)
            .arg(self.max_requests)
            .arg(window_seconds)
            .invoke(&mut conn);

        match result {
            Ok(1) => Ok(()),
            Ok(0) => Err(RateLimiterError::RateLimitExceeded),
            Ok(_) => Ok(()), // Any other value means we're under the limit
            Err(e) => Err(RateLimiterError::Redis(e)),
        }
    }

    pub fn get_remaining(&self, identifier: &str) -> Result<u64, RateLimiterError> {
        let key = self.get_redis_key(identifier);
        let mut conn = self.redis_client.get_connection()?;
        let count: Option<u64> = conn.get(&key)?;
        Ok(self.max_requests.saturating_sub(count.unwrap_or(0)))
    }

    pub fn get_time_remaining(&self, identifier: &str) -> Result<i64, RateLimiterError> {
        let key = self.get_redis_key(identifier);
        let mut conn = self.redis_client.get_connection()?;
        let ttl: i64 = conn.ttl(&key)?;
        Ok(if ttl == -2 { -1 } else { ttl })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::sync::Mutex;

    const REDIS_URL: &str = "redis://127.0.0.1:6379";

    // Simple counter for generating unique prefixes
    static PREFIX_COUNTER: Mutex<u32> = Mutex::new(0);

    fn get_unique_prefix() -> String {
        let mut counter = PREFIX_COUNTER.lock().unwrap();
        *counter += 1;
        format!("test_rate_limiter_{}", *counter)
    }

    #[test]
    fn test_basic_rate_limiting() -> Result<(), RateLimiterError> {
        let prefix = get_unique_prefix();
        let limiter = RateLimiter::new(REDIS_URL, &prefix, 3, Duration::from_secs(1))?;
        let identifier = "user_1";

        assert!(limiter.check(identifier).is_ok());
        assert!(limiter.check(identifier).is_ok());
        assert!(limiter.check(identifier).is_ok());
        assert!(limiter.check(identifier).is_err()); // Rate limit exceeded

        sleep(Duration::from_secs(2)); // Wait for the window to expire

        assert!(limiter.check(identifier).is_ok()); // Should allow again

        Ok(())
    }

    #[test]
    fn test_multiple_identifiers() -> Result<(), RateLimiterError> {
        let prefix = get_unique_prefix();
        let limiter = RateLimiter::new(REDIS_URL, &prefix, 2, Duration::from_secs(1))?;
        let user_1 = "user_1";
        let user_2 = "user_2";

        assert!(limiter.check(user_1).is_ok());
        assert!(limiter.check(user_2).is_ok());
        assert!(limiter.check(user_1).is_ok());
        assert!(limiter.check(user_2).is_ok());
        assert!(limiter.check(user_1).is_err());
        assert!(limiter.check(user_2).is_err());

        sleep(Duration::from_secs(2));

        assert!(limiter.check(user_1).is_ok());
        assert!(limiter.check(user_2).is_ok());

        Ok(())
    }

    #[test]
    fn test_get_remaining() -> Result<(), RateLimiterError> {
        let prefix = get_unique_prefix();
        let limiter = RateLimiter::new(REDIS_URL, &prefix, 5, Duration::from_secs(5))?;
        let identifier = "user_3";

        assert_eq!(limiter.get_remaining(identifier)?, 5);
        assert!(limiter.check(identifier).is_ok());
        assert_eq!(limiter.get_remaining(identifier)?, 4);
        assert!(limiter.check(identifier).is_ok());
        assert_eq!(limiter.get_remaining(identifier)?, 3);

        Ok(())
    }

    #[test]
    fn test_get_time_remaining() -> Result<(), RateLimiterError> {
        let prefix = get_unique_prefix();
        let limiter = RateLimiter::new(REDIS_URL, &prefix, 2, Duration::from_secs(3))?;
        let identifier = "user_4";

        assert!(limiter.check(identifier).is_ok());
        let ttl1 = limiter.get_time_remaining(identifier)?;
        assert!(ttl1 > 0 && ttl1 <= 3);

        sleep(Duration::from_secs(2));
        let ttl2 = limiter.get_time_remaining(identifier)?;
        assert!(ttl2 >= 0 && ttl2 <= 1);

        sleep(Duration::from_secs(2));
        let ttl3 = limiter.get_time_remaining(identifier)?;
        assert_eq!(ttl3, -1); // Key should have expired

        Ok(())
    }
}
