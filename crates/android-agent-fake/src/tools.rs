//! Phase 0 模拟工具实现：返回结构稳定的假数据，供协议与策略链路验证。

use hermes_core::protocol::{Status, ToolRequest, ToolResponse};
use serde_json::{json, Value};

use crate::sandbox::{check_path, PathVerdict};

/// 工具名常量。
pub mod names {
    pub const DEVICE_INFO: &str = "android.device_info";
    pub const BATTERY_STATUS: &str = "android.battery_status";
    pub const NETWORK_STATUS: &str = "android.network_status";
    pub const STORAGE_STATUS: &str = "android.storage_status";
    pub const PROCESS_LIST: &str = "android.process_list";
    pub const THERMAL_STATUS: &str = "android.thermal_status";
    pub const APP_LIST: &str = "android.app.list";
    pub const APP_LAUNCH: &str = "android.app.launch";
    pub const SCREEN_CAPTURE: &str = "android.screen.capture";
    pub const UI_TREE: &str = "android.ui.tree";
    pub const FILE_LIST: &str = "android.file.list";
}

/// 本 Agent 支持的全部工具。
pub const SUPPORTED_TOOLS: &[&str] = &[
    names::DEVICE_INFO,
    names::BATTERY_STATUS,
    names::NETWORK_STATUS,
    names::STORAGE_STATUS,
    names::PROCESS_LIST,
    names::THERMAL_STATUS,
    names::APP_LIST,
    names::APP_LAUNCH,
    names::SCREEN_CAPTURE,
    names::UI_TREE,
    names::FILE_LIST,
];

/// 构造成功响应。
fn success(request: &ToolRequest, value: Value) -> ToolResponse {
    ToolResponse {
        request_id: request.request_id.clone(),
        status: Status::Success,
        tool: request.tool.clone(),
        result: Some(value),
        audit_id: None,
        duration_ms: 1,
        message: None,
    }
}

/// 执行一个已通过 Schema 校验的请求，返回标准化响应。
pub fn execute(request: &ToolRequest) -> ToolResponse {
    match request.tool.as_str() {
        names::DEVICE_INFO => success(
            request,
            json!({
                "model": "Hermes Virtual Device",
                "android_version": "14",
                "sdk_version": 34,
                "battery_level": 87,
                "network_type": "wifi",
                "screen_width": 1080,
                "screen_height": 2400,
                "foreground_package": "com.example.launcher"
            }),
        ),
        names::BATTERY_STATUS => success(
            request,
            json!({
                "level_percent": 87,
                "charging": true,
                "temperature_c": 31.5,
                "health": "good"
            }),
        ),
        names::NETWORK_STATUS => success(
            request,
            json!({
                "connected": true,
                "type": "wifi",
                "ssid": "hermes-lab",
                "ip": "192.168.1.42"
            }),
        ),
        names::STORAGE_STATUS => success(
            request,
            json!({
                "total_bytes": 128_000_000_000u64,
                "free_bytes": 42_000_000_000u64,
                "mounts": ["/data", "/sdcard"]
            }),
        ),
        names::PROCESS_LIST => success(
            request,
            json!({
                "processes": [
                    { "pid": 1024, "name": "system_server", "user": "system" },
                    { "pid": 2048, "name": "com.example.launcher", "user": "u0_a101" }
                ]
            }),
        ),
        names::THERMAL_STATUS => success(
            request,
            json!({
                "zones": [
                    { "name": "cpu-0-1", "temperature_c": 44.2, "throttling": false },
                    { "name": "battery", "temperature_c": 31.5, "throttling": false }
                ]
            }),
        ),
        names::APP_LIST => success(
            request,
            json!({
                "packages": [
                    { "package_name": "com.android.settings", "label": "设置", "enabled": true },
                    { "package_name": "com.example.launcher", "label": "启动器", "enabled": true }
                ]
            }),
        ),
        names::APP_LAUNCH => {
            let package = request.arguments["package_name"].as_str().unwrap_or_default();
            success(
                request,
                json!({
                    "package_name": package,
                    "activity": "MainActivity",
                    "started": true
                }),
            )
        }
        names::SCREEN_CAPTURE => success(
            request,
            json!({
                "width": 1080,
                "height": 2400,
                "orientation": "portrait",
                "format": "png",
                "data_base64": "iVBORw0KGgo="
            }),
        ),
        names::UI_TREE => success(
            request,
            json!({
                "root": {
                    "class": "android.widget.FrameLayout",
                    "resource_id": "android:id/content",
                    "bounds": [0, 0, 1080, 2400],
                    "children": [
                        {
                            "class": "android.widget.TextView",
                            "text": "Hermes",
                            "resource_id": "com.example:id/title",
                            "bounds": [48, 120, 400, 200],
                            "children": []
                        }
                    ]
                }
            }),
        ),
        names::FILE_LIST => {
            let path = request.arguments["path"].as_str().unwrap_or("/");
            match check_path(path) {
                PathVerdict::Allowed => success(
                    request,
                    json!({
                        "path": path,
                        "entries": [
                            { "name": "sample.txt", "type": "file", "size_bytes": 128 },
                            { "name": "photos", "type": "dir" }
                        ]
                    }),
                ),
                PathVerdict::Denied => ToolResponse::failure(
                    &request.request_id,
                    &request.tool,
                    Status::PolicyBlocked,
                    format!("路径 {path} 不在沙箱白名单内"),
                ),
            }
        }
        other => ToolResponse::failure(
            &request.request_id,
            &request.tool,
            Status::ExecutionFailed,
            format!("模拟 Agent 不支持工具 {other}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tool: &str, arguments: Value) -> ToolRequest {
        ToolRequest {
            request_id: "req_t".to_string(),
            session_id: String::new(),
            device_id: "device_001".to_string(),
            tool: tool.to_string(),
            arguments,
            requested_by: None,
            reason: String::new(),
            approval_token: None,
        }
    }

    #[test]
    fn every_supported_tool_returns_success_with_result() {
        for tool in SUPPORTED_TOOLS {
            let arguments = match *tool {
                names::APP_LAUNCH => json!({ "package_name": "com.android.settings" }),
                names::FILE_LIST => json!({ "path": "/sdcard/Download" }),
                _ => json!({}),
            };
            let resp = execute(&request(tool, arguments));
            assert!(resp.status.is_success(), "{tool} -> {:?}", resp.status);
            assert!(resp.result.is_some(), "{tool} 缺少 result");
        }
    }

    #[test]
    fn app_launch_echoes_package_and_reports_started() {
        let resp = execute(&request(
            names::APP_LAUNCH,
            json!({ "package_name": "com.android.settings" }),
        ));
        let result = resp.result.unwrap();
        assert_eq!(result["package_name"], json!("com.android.settings"));
        assert_eq!(result["started"], json!(true));
        assert!(result["activity"].is_string());
    }

    #[test]
    fn file_list_denies_out_of_sandbox_path() {
        let resp = execute(&request(names::FILE_LIST, json!({ "path": "/system/etc" })));
        assert_eq!(resp.status, Status::PolicyBlocked);
        assert!(resp.result.is_none());
    }

    #[test]
    fn unknown_tool_fails_without_panic() {
        let resp = execute(&request("android.shell.exec", json!({})));
        assert_eq!(resp.status, Status::ExecutionFailed);
    }
}
