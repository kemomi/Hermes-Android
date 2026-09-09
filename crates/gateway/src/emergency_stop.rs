//! 紧急停止：全局开关，启用后拒绝所有高风险（非只读）调用。

use std::sync::atomic::{AtomicBool, Ordering};

/// 线程安全紧急停止开关。
#[derive(Debug, Default)]
pub struct EmergencyStop {
    engaged: AtomicBool,
}

impl EmergencyStop {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启用紧急停止。
    pub fn engage(&self) {
        self.engaged.store(true, Ordering::SeqCst);
    }

    /// 解除紧急停止。
    pub fn release(&self) {
        self.engaged.store(false, Ordering::SeqCst);
    }

    /// 当前是否处于紧急停止状态。
    pub fn is_engaged(&self) -> bool {
        self.engaged.load(Ordering::SeqCst)
    }

    /// 判断一次调用是否被紧急停止拦截。
    ///
    /// 策略：紧急停止启用时，只读工具仍放行（便于诊断），
    /// 非只读 / 未知工具一律拒绝。
    pub fn blocks(&self, read_only: Option<bool>) -> bool {
        self.is_engaged() && read_only != Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disengaged_by_default() {
        let stop = EmergencyStop::new();
        assert!(!stop.is_engaged());
        assert!(!stop.blocks(Some(false)));
        assert!(!stop.blocks(None));
    }

    #[test]
    fn engages_and_blocks_non_readonly() {
        let stop = EmergencyStop::new();
        stop.engage();
        assert!(stop.is_engaged());
        assert!(stop.blocks(Some(false)), "写操作应被拦截");
        assert!(stop.blocks(None), "未知只读性应被拦截");
        assert!(!stop.blocks(Some(true)), "只读操作仍放行");
    }

    #[test]
    fn release_restores_normal_operation() {
        let stop = EmergencyStop::new();
        stop.engage();
        assert!(stop.blocks(Some(false)));
        stop.release();
        assert!(!stop.blocks(Some(false)));
    }
}
