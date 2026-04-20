use std::{collections::HashMap, sync::Mutex, time::Instant};

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Token Bucket rate limiter keyed by `chat_id`.
/// Parameters: capacity=5 tokens, refill_rate=5 tokens/sec (BR-055).
/// LRU eviction: keys inactive for > `eviction_secs` are removed on each call.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    capacity: f64,
    refill_rate: f64,
    eviction_secs: f64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::new_with_config(5.0, 5.0, 60.0)
    }

    pub fn new_with_config(capacity: f64, refill_rate: f64, eviction_secs: f64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            capacity,
            refill_rate,
            eviction_secs,
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    pub fn allow(&self, chat_id: &str) -> bool {
        let mut map = self.buckets.lock().unwrap();

        let eviction_secs = self.eviction_secs;
        let now = Instant::now();
        map.retain(|_, bucket| {
            now.duration_since(bucket.last_refill).as_secs_f64() < eviction_secs
        });

        let bucket = map
            .entry(chat_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.allow()
    }

    #[cfg(test)]
    fn bucket_count(&self) -> usize {
        self.buckets.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn five_consecutive_requests_allowed() {
        let rl = RateLimiter::new();
        for _ in 0..5 {
            assert!(rl.allow("chat-a"));
        }
    }

    #[test]
    fn sixth_request_rate_limited() {
        let rl = RateLimiter::new();
        for _ in 0..5 {
            rl.allow("chat-a");
        }
        assert!(!rl.allow("chat-a"));
    }

    #[test]
    fn different_chat_ids_are_independent() {
        let rl = RateLimiter::new();
        for _ in 0..5 {
            rl.allow("chat-a");
        }
        assert!(!rl.allow("chat-a"), "chat-a should be limited");
        assert!(rl.allow("chat-b"), "chat-b should not be affected");
    }

    #[test]
    fn lru_evicts_inactive_keys() {
        let rl = RateLimiter::new_with_config(5.0, 5.0, 0.1); // 100ms eviction
        rl.allow("chat-a");
        assert_eq!(rl.bucket_count(), 1);
        std::thread::sleep(Duration::from_millis(150));
        rl.allow("chat-b"); // triggers cleanup
        assert_eq!(
            rl.bucket_count(),
            1,
            "stale chat-a should have been evicted"
        );
    }
}
