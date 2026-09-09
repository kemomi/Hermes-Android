//! 审批管理：为高风险工具签发一次性 approval_token，校验并消费令牌。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

/// 待审批请求的元数据。
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub request_id: String,
    pub tool: String,
    pub reason: String,
}

/// 线程安全审批管理器。
#[derive(Debug, Default)]
pub struct ApprovalManager {
    /// token -> 授权的工具名（一次性消费）。
    tokens: Mutex<HashMap<String, String>>,
    counter: AtomicU64,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 为待审批请求签发令牌。
    pub fn issue(&self, pending: &PendingApproval) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        let seed = format!("{}|{}|{n}", pending.request_id, pending.tool);
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let token: String = hasher
            .finalize()
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        self.tokens
            .lock()
            .expect("审批锁中毒")
            .insert(token.clone(), pending.tool.clone());
        token
    }

    /// 校验并消费令牌：仅当令牌存在且工具匹配时返回 true（一次性）。
    pub fn consume(&self, token: &str, tool: &str) -> bool {
        let mut tokens = self.tokens.lock().expect("审批锁中毒");
        match tokens.get(token) {
            Some(allowed) if allowed == tool => {
                tokens.remove(token);
                true
            }
            _ => false,
        }
    }

    /// 当前未消费令牌数量。
    pub fn pending_count(&self) -> usize {
        self.tokens.lock().expect("审批锁中毒").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> PendingApproval {
        PendingApproval {
            request_id: "req_h".into(),
            tool: "android.file.delete".into(),
            reason: "删除临时文件".into(),
        }
    }

    #[test]
    fn issued_token_consumes_once_for_matching_tool() {
        let mgr = ApprovalManager::new();
        let token = mgr.issue(&pending());
        assert_eq!(mgr.pending_count(), 1);
        assert!(mgr.consume(&token, "android.file.delete"));
        assert_eq!(mgr.pending_count(), 0);
        // 二次消费失败（一次性）
        assert!(!mgr.consume(&token, "android.file.delete"));
    }

    #[test]
    fn token_rejected_for_wrong_tool() {
        let mgr = ApprovalManager::new();
        let token = mgr.issue(&pending());
        assert!(!mgr.consume(&token, "android.app.launch"));
        // 工具不匹配时令牌不被消费
        assert_eq!(mgr.pending_count(), 1);
    }

    #[test]
    fn unknown_token_rejected() {
        let mgr = ApprovalManager::new();
        assert!(!mgr.consume("bogus", "android.file.delete"));
    }

    #[test]
    fn distinct_tokens_per_issue() {
        let mgr = ApprovalManager::new();
        let a = mgr.issue(&pending());
        let b = mgr.issue(&pending());
        assert_ne!(a, b);
    }
}
