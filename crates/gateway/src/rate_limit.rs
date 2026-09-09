//! 限频器：滑动时间窗内限制单设备的调用次数，超限返回 RATE 限制（映射 FORBIDDEN）。

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 线程安全滑动窗口限频器（按 device_id 计数）。
#[derive(Debug)]
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    hits: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        RateLimiter {
            max_requests,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// 尝试为某设备记录一次调用；返回 false 表示已超出窗口配额。
    pub fn try_acquire(&self, device_id: &str, now: Instant) -> bool {
        let mut hits = self.hits.lock().expect("限频锁中毒");
        let queue = hits.entry(device_id.to_string()).or_default();
        // 淘汰窗口外的旧记录
        while let Some(front) = queue.front() {
            if now.duration_since(*front) >= self.window {
                queue.pop_front();
            } else {
                break;
            }
        }
        if queue.len() >= self.max_requests {
            return false;
        }
        queue.push_back(now);
        true
    }

    /// 便捷方法：使用当前时刻。
    pub fn acquire(&self, device_id: &str) -> bool {
        self.try_acquire(device_id, Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_within_window() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let now = Instant::now();
        assert!(limiter.try_acquire("dev", now));
        assert!(limiter.try_acquire("dev", now));
        assert!(limiter.try_acquire("dev", now));
        assert!(!limiter.try_acquire("dev", now), "第 4 次应被限频");
    }

    #[test]
    fn different_devices_have_separate_budgets() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();
        assert!(limiter.try_acquire("a", now));
        assert!(limiter.try_acquire("b", now));
        assert!(!limiter.try_acquire("a", now));
    }

    #[test]
    fn budget_recovers_after_window_elapses() {
        let limiter = RateLimiter::new(2, Duration::from_secs(10));
        let now = Instant::now();
        assert!(limiter.try_acquire("dev", now));
        assert!(limiter.try_acquire("dev", now));
        assert!(!limiter.try_acquire("dev", now));
        // 模拟窗口流逝
        let later = now + Duration::from_secs(11);
        assert!(limiter.try_acquire("dev", later));
    }
}
