use std::time::Duration;

use insights_bot_telegram_rs::services::rate_limit::{CommandRateLimiter, RateKey};

#[test]
fn rate_limit_blocks_second_request_within_window() {
    let limiter = CommandRateLimiter::new(1, Duration::from_millis(200));
    let key = RateKey(1, "recap");
    assert!(limiter.check(key.clone()).is_ok());
    assert!(limiter.check(key.clone()).is_err());
    // after window, should allow again
    std::thread::sleep(Duration::from_millis(220));
    assert!(limiter.check(key).is_ok());
}
