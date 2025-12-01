use std::{num::NonZeroU32, sync::Arc};

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
    pub fn new(ops_per_sec: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(ops_per_sec).unwrap_or(nonzero!(1u32)));
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
