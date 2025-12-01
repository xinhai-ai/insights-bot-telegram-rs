use std::{num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Result;
use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware,
    state::keyed::DefaultKeyedStateStore,
};
use nonzero_ext::nonzero;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RateKey(pub i64, pub &'static str);

#[derive(Clone)]
pub struct CommandRateLimiter {
    limiter:
        Arc<RateLimiter<RateKey, DefaultKeyedStateStore<RateKey>, DefaultClock, NoOpMiddleware>>,
}

impl CommandRateLimiter {
    pub fn new(ops_per_window: u32, window: Duration) -> Self {
        let quota = Quota::with_period(window)
            .unwrap()
            .allow_burst(NonZeroU32::new(ops_per_window).unwrap_or(nonzero!(1u32)));
        Self {
            limiter: Arc::new(RateLimiter::keyed(quota)),
        }
    }

    pub fn check(&self, key: RateKey) -> Result<()> {
        self.limiter
            .check_key(&key)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }
}
