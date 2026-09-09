//! 统一调用协议：POST /v1/tools/execute 的请求/响应与标准化失败状态。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 调用发起方身份。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestedBy {
    #[serde(rename = "type")]
    pub kind: String,
    pub user_id: String,
}

/// 工具调用请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRequest {
    pub request_id: String,
    #[serde(default)]
    pub session_id: String,
    pub device_id: String,
    pub tool: String,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default)]
    pub requested_by: Option<RequestedBy>,
    #[serde(default)]
    pub reason: String,
    /// 人工审批令牌（高风险工具需要）。
    #[serde(default)]
    pub approval_token: Option<String>,
}

/// 标准化执行状态：success 或蓝图定义的 10 种失败码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    #[serde(rename = "success")]
    Success,
    InvalidArgument,
    Unauthorized,
    Forbidden,
    ApprovalRequired,
    DeviceOffline,
    Timeout,
    ExecutionFailed,
    PolicyBlocked,
    DuplicateRequest,
    RollbackRequired,
}

impl Status {
    pub fn is_success(self) -> bool {
        matches!(self, Status::Success)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Success => "success",
            Status::InvalidArgument => "INVALID_ARGUMENT",
            Status::Unauthorized => "UNAUTHORIZED",
            Status::Forbidden => "FORBIDDEN",
            Status::ApprovalRequired => "APPROVAL_REQUIRED",
            Status::DeviceOffline => "DEVICE_OFFLINE",
            Status::Timeout => "TIMEOUT",
            Status::ExecutionFailed => "EXECUTION_FAILED",
            Status::PolicyBlocked => "POLICY_BLOCKED",
            Status::DuplicateRequest => "DUPLICATE_REQUEST",
            Status::RollbackRequired => "ROLLBACK_REQUIRED",
        }
    }
}

/// 工具调用响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponse {
    pub request_id: String,
    pub status: Status,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_id: Option<String>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ToolResponse {
    pub fn failure(request_id: &str, tool: &str, status: Status, message: impl Into<String>) -> Self {
        ToolResponse {
            request_id: request_id.to_string(),
            status,
            tool: tool.to_string(),
            result: None,
            audit_id: None,
            duration_ms: 0,
            message: Some(message.into()),
        }
    }
}

/// 设备主动上报的事件（见 events 模块的载荷约定）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEvent {
    pub event_id: String,
    pub event_type: String,
    pub device_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub payload: Value,
    /// normal | sensitive；sensitive 事件只发元数据，不发完整内容。
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
}

fn default_sensitivity() -> String {
    "normal".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip_matches_blueprint_example() {
        let json = r#"{
            "request_id": "req_001",
            "session_id": "sess_001",
            "device_id": "device_001",
            "tool": "android.app.launch",
            "arguments": { "package_name": "com.android.settings" },
            "requested_by": { "type": "hermes", "user_id": "user_001" },
            "reason": "打开系统设置"
        }"#;
        let req: ToolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_id, "req_001");
        assert_eq!(req.tool, "android.app.launch");
        assert_eq!(
            req.arguments["package_name"].as_str(),
            Some("com.android.settings")
        );
        assert_eq!(req.requested_by.as_ref().unwrap().kind, "hermes");
        // 序列化再反序列化保持稳定
        let again: ToolRequest = serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert_eq!(req, again);
    }

    #[test]
    fn status_codes_serialize_as_blueprint_names() {
        assert_eq!(serde_json::to_string(&Status::Success).unwrap(), r#""success""#);
        assert_eq!(
            serde_json::to_string(&Status::ApprovalRequired).unwrap(),
            r#""APPROVAL_REQUIRED""#
        );
        assert_eq!(
            serde_json::to_string(&Status::DuplicateRequest).unwrap(),
            r#""DUPLICATE_REQUEST""#
        );
        let parsed: Status = serde_json::from_str(r#""POLICY_BLOCKED""#).unwrap();
        assert_eq!(parsed, Status::PolicyBlocked);
    }

    #[test]
    fn response_failure_has_message_and_no_result() {
        let resp = ToolResponse::failure("req_9", "android.file.list", Status::PolicyBlocked, "路径越权");
        assert!(resp.result.is_none());
        assert_eq!(resp.message.as_deref(), Some("路径越权"));
        assert!(!resp.status.is_success());
    }
}
