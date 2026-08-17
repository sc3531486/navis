//! 与业务无关的可靠性原语：退避、重试预算、超时和并发安全熔断。

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitRejected;

#[derive(Debug, Clone)]
pub struct Backoff {
    pub max_retries: u32,
    pub initial: Duration,
    pub maximum: Duration,
    pub multiplier: f64,
}

impl Backoff {
    pub fn delay(&self, attempt: u32) -> Duration {
        let millis = self.initial.as_secs_f64()
            * 1000.0
            * self.multiplier.powi(attempt.min(i32::MAX as u32) as i32);
        Duration::from_millis(millis.min(self.maximum.as_secs_f64() * 1000.0) as u64)
    }

    pub const fn can_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial: Duration::from_millis(100),
            maximum: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeoutPolicy {
    pub default: Duration,
    pub maximum: Duration,
}

impl TimeoutPolicy {
    pub const fn new(default: Duration, maximum: Duration) -> Self {
        Self { default, maximum }
    }

    pub fn effective(self, requested: Option<Duration>) -> Duration {
        match requested {
            Some(value) if !value.is_zero() && value <= self.maximum => value,
            _ => self.default,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

struct CircuitInner {
    state: CircuitState,
    failures: u32,
    opened_at: Option<Instant>,
    half_open_in_flight: u32,
}

/// 并发安全的熔断器。半开请求以 permit 计数，结果必须归还给同一个 permit。
#[derive(Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Arc<Mutex<CircuitInner>>,
}

pub struct CircuitPermit {
    breaker: CircuitBreaker,
    half_open: bool,
    completed: bool,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(CircuitInner {
                state: CircuitState::Closed,
                failures: 0,
                opened_at: None,
                half_open_in_flight: 0,
            })),
        }
    }

    pub fn acquire(&self) -> Result<CircuitPermit, CircuitRejected> {
        let mut inner = self.inner.lock().expect("circuit breaker mutex poisoned");
        if inner.state == CircuitState::Open {
            let recovered = inner
                .opened_at
                .is_some_and(|opened_at| opened_at.elapsed() >= self.config.recovery_timeout);
            if recovered {
                inner.state = CircuitState::HalfOpen;
                inner.half_open_in_flight = 0;
            }
        }

        let half_open = inner.state == CircuitState::HalfOpen;
        if inner.state == CircuitState::Open
            || (half_open && inner.half_open_in_flight >= self.config.half_open_max_requests)
        {
            return Err(CircuitRejected);
        }
        if half_open {
            inner.half_open_in_flight += 1;
        }
        Ok(CircuitPermit {
            breaker: self.clone(),
            half_open,
            completed: false,
        })
    }

    pub fn state(&self) -> CircuitState {
        self.inner
            .lock()
            .expect("circuit breaker mutex poisoned")
            .state
    }

    pub fn failures(&self) -> u32 {
        self.inner
            .lock()
            .expect("circuit breaker mutex poisoned")
            .failures
    }

    pub fn reset(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker mutex poisoned");
        inner.state = CircuitState::Closed;
        inner.failures = 0;
        inner.opened_at = None;
        inner.half_open_in_flight = 0;
    }

    fn complete(&self, half_open: bool, success: bool) {
        let mut inner = self.inner.lock().expect("circuit breaker mutex poisoned");
        if half_open {
            inner.half_open_in_flight = inner.half_open_in_flight.saturating_sub(1);
        }
        if success {
            inner.failures = 0;
            if half_open {
                inner.state = CircuitState::Closed;
                inner.opened_at = None;
            }
        } else {
            inner.failures = inner.failures.saturating_add(1);
            inner.state = CircuitState::Open;
            inner.opened_at = Some(Instant::now());
        }
    }
}

impl CircuitPermit {
    pub fn success(mut self) {
        self.completed = true;
        self.breaker.complete(self.half_open, true);
    }

    pub fn failure(mut self) {
        self.completed = true;
        self.breaker.complete(self.half_open, false);
    }
}

impl Drop for CircuitPermit {
    fn drop(&mut self) {
        if !self.completed {
            self.breaker.complete(self.half_open, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_exponential_and_capped() {
        let backoff = Backoff {
            maximum: Duration::from_millis(250),
            ..Default::default()
        };
        assert_eq!(backoff.delay(0), Duration::from_millis(100));
        assert_eq!(backoff.delay(1), Duration::from_millis(200));
        assert_eq!(backoff.delay(2), Duration::from_millis(250));
    }

    #[test]
    fn half_open_permits_are_bounded_and_recover() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::ZERO,
            half_open_max_requests: 1,
        });
        breaker.acquire().unwrap().failure();
        let probe = breaker.acquire().unwrap();
        assert!(breaker.acquire().is_err());
        probe.success();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.acquire().is_ok());
    }
}
