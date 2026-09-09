//! 设备事件模型与推荐事件类型。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 蓝图推荐的事件类型常量。
pub mod event_type {
    pub const DEVICE_CONNECTED: &str = "device.connected";
    pub const DEVICE_DISCONNECTED: &str = "device.disconnected";
    pub const APP_FOREGROUND_CHANGED: &str = "android.app.foreground_changed";
    pub const NOTIFICATION_RECEIVED: &str = "android.notification.received";
    pub const FILE_CREATED: &str = "android.file.created";
    pub const FILE_CHANGED: &str = "android.file.changed";
    pub const SCREEN_CHANGED: &str = "android.screen.changed";
    pub const ACTION_COMPLETED: &str = "android.action.completed";
    pub const ACTION_FAILED: &str = "android.action.failed";
    pub const KERNEL_THERMAL_WARNING: &str = "kernel.thermal.warning";
    pub const KERNEL_PROCESS_EVENT: &str = "kernel.process_event";
}

/// 事件敏感度：sensitive 事件只允许携带元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Normal,
    Sensitive,
}

/// 设备主动上报事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub event_type: String,
    pub device_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub payload: Value,
    pub sensitivity: Sensitivity,
}

impl Event {
    /// 构造事件；sensitive 事件强制裁剪 payload 为元数据（键数≤4 且不含大字段）。
    pub fn new(
        event_id: impl Into<String>,
        event_type: impl Into<String>,
        device_id: impl Into<String>,
        payload: Value,
        sensitivity: Sensitivity,
    ) -> Self {
        let payload = match sensitivity {
            Sensitivity::Normal => payload,
            Sensitivity::Sensitive => trim_to_metadata(payload),
        };
        Event {
            event_id: event_id.into(),
            event_type: event_type.into(),
            device_id: device_id.into(),
            timestamp: crate::audit::now_iso8601(),
            payload,
            sensitivity,
        }
    }
}

/// 敏感事件裁剪：只保留最多 4 个标量字段，丢弃字符串内容体。
fn trim_to_metadata(payload: Value) -> Value {
    match payload {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map.into_iter().take(4) {
                if v.is_object() || v.is_array() {
                    continue;
                }
                let v = match v {
                    Value::String(s) if s.chars().count() > 64 => {
                        let truncated: String = s.chars().take(60).collect();
                        Value::String(format!("{truncated}…"))
                    }
                    other => other,
                };
                out.insert(k, v);
            }
            Value::Object(out)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sensitive_event_payload_is_trimmed() {
        let evt = Event::new(
            "evt_001",
            event_type::NOTIFICATION_RECEIVED,
            "device_001",
            json!({
                "package_name": "com.example.chat",
                "title": "一条非常长的通知标题内容用于验证裁剪逻辑是否生效的文本内容",
                "body": "x".repeat(500),
                "post_time": 1757200000,
                "extra": { "nested": "should be dropped" },
                "more": "dropped"
            }),
            Sensitivity::Sensitive,
        );
        let obj = evt.payload.as_object().unwrap();
        assert!(obj.len() <= 4);
        assert!(!obj.contains_key("extra"));
        let body = obj["body"].as_str().unwrap();
        assert!(body.len() <= 64);
    }

    #[test]
    fn normal_event_keeps_payload() {
        let evt = Event::new(
            "evt_002",
            event_type::APP_FOREGROUND_CHANGED,
            "device_001",
            json!({ "package_name": "com.example.app", "activity": "MainActivity" }),
            Sensitivity::Normal,
        );
        assert_eq!(evt.payload["activity"], json!("MainActivity"));
    }

    #[test]
    fn event_serializes_with_blueprint_field_names() {
        let evt = Event::new("evt_003", event_type::DEVICE_CONNECTED, "device_001", json!({}), Sensitivity::Normal);
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event_id\""));
        assert!(text.contains("\"event_type\""));
        assert!(text.contains("\"sensitivity\""));
    }
}
