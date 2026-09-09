//! 幂等存储：按 request_id 去重，命中重复请求返回 DUPLICATE_REQUEST 与首次结果。

use std::collections::HashMap;
use std::sync::Mutex;

use hermes_core::protocol::ToolResponse;

/// 线程安全的内存幂等存储（进程级；持久化为后续阶段目标）。
#[derive(Debug, Default)]
pub struct IdempotencyStore {
    seen: Mutex<HashMap<String, ToolResponse>>,
}

/// 记录一次请求的结果。
pub enum IdempotencyOutcome {
    /// 首次出现，调用方应执行工具并随后 `complete`。
    New,
    /// 重复请求，携带首次结果。
    Duplicate(ToolResponse),
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查 request_id 是否已处理；未处理则占位为「进行中」不可见。
    pub fn check(&self, request_id: &str) -> IdempotencyOutcome {
        let map = self.seen.lock().expect("幂等存储锁中毒");
        match map.get(request_id) {
            Some(resp) => IdempotencyOutcome::Duplicate(resp.clone()),
            None => IdempotencyOutcome::New,
        }
    }

    /// 写入首次执行结果，供后续重复请求复用。
    pub fn complete(&self, request_id: &str, response: &ToolResponse) {
        let mut map = self.seen.lock().expect("幂等存储锁中毒");
        map.insert(request_id.to_string(), response.clone());
    }

    pub fn len(&self) -> usize {
        self.seen.lock().expect("幂等存储锁中毒").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::protocol::{Status, ToolResponse};

    #[test]
    fn first_request_is_new_then_duplicate() {
        let store = IdempotencyStore::new();
        assert!(matches!(store.check("req_1"), IdempotencyOutcome::New));
        let resp = ToolResponse {
            request_id: "req_1".into(),
            status: Status::Success,
            tool: "android.device_info".into(),
            result: Some(serde_json::json!({"ok": true})),
            audit_id: Some("audit_000001".into()),
            duration_ms: 3,
            message: None,
        };
        store.complete("req_1", &resp);
        match store.check("req_1") {
            IdempotencyOutcome::Duplicate(d) => assert_eq!(d, resp),
            IdempotencyOutcome::New => panic!("应判定为重复"),
        }
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn different_ids_do_not_collide() {
        let store = IdempotencyStore::new();
        assert!(matches!(store.check("a"), IdempotencyOutcome::New));
        assert!(matches!(store.check("b"), IdempotencyOutcome::New));
    }
}
