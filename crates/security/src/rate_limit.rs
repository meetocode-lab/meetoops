use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple, bounded sliding-window rate limiter for auth, setup, and token endpoints.
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    buckets: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut guard = self.buckets.lock().expect("rate limiter mutex");

        // Opportunistic cleanup to keep memory bounded
        if guard.len() > 4096 {
            guard.retain(|_, hits| {
                hits.retain(|t| now.duration_since(*t) < self.window);
                !hits.is_empty()
            });
        }

        let hits = guard.entry(key.to_string()).or_default();
        hits.retain(|t| now.duration_since(*t) < self.window);
        if hits.len() >= self.max_requests as usize {
            return false;
        }
        hits.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_excess_requests_within_window() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("127.0.0.1"));
        assert!(limiter.check("127.0.0.1"));
        assert!(!limiter.check("127.0.0.1"));
        assert!(limiter.check("10.0.0.1"));
    }
}
