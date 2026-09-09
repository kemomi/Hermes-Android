//! 端到端集成测试：in-process 启动 fake Android Agent 与 Gateway，
//! 通过真实 HTTP 接口验证蓝图验收标准的全部关键场景。

use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gateway::http_server::{self, HttpServerConfig};
use gateway::policy::{default_policy, PolicyConfig};
use gateway::router::AgentEndpoint;
use gateway::Gateway;
use hermes_core::audit::AuditLogger;
use hermes_core::http_client;
use hermes_core::protocol::{RequestedBy, ToolRequest};
use serde_json::{json, Value};

const TIMEOUT: Duration = Duration::from_secs(10);
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn unique_tmp(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "hermes_e2e_{tag}_{}_{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 启动 fake agent，返回其监听地址。
fn spawn_agent() -> SocketAddr {
    android_agent_fake::serve(0).expect("启动 fake agent 失败")
}

/// 启动一个「黑洞」TCP 服务：接受连接但不回复，用于触发执行超时。
fn spawn_blackhole() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(s) = stream {
                std::thread::spawn(move || {
                    let _keep = s;
                    std::thread::sleep(Duration::from_secs(15));
                });
            }
        }
    });
    addr
}

/// 启动 Gateway（含 HTTP 服务），返回其 HTTP 地址与审计文件路径。
fn spawn_gateway(
    policy: PolicyConfig,
    agent_addr: SocketAddr,
    rate_limit: usize,
) -> (SocketAddr, PathBuf) {
    let tmp = unique_tmp("gw");
    let audit_path = tmp.join("audit.jsonl");
    let skills = repo_root().join("skills");
    let endpoint = AgentEndpoint {
        addr: agent_addr,
        connect_timeout: Duration::from_millis(500),
    };
    let gateway = Gateway::new(
        &skills,
        policy,
        endpoint,
        &audit_path,
        rate_limit,
        Duration::from_secs(60),
    )
    .expect("初始化 Gateway 失败");

    let (addr, listener) =
        http_server::bind(HttpServerConfig { bind: "127.0.0.1:0".parse().unwrap() })
            .expect("绑定 HTTP 失败");
    std::thread::spawn(move || http_server::serve_with_listener(listener, Arc::new(gateway)));
    (addr, audit_path)
}

fn make_request(tool: &str, args: Value, request_id: &str, token: Option<String>) -> ToolRequest {
    ToolRequest {
        request_id: request_id.to_string(),
        session_id: "sess_e2e".to_string(),
        device_id: "device_001".to_string(),
        tool: tool.to_string(),
        arguments: args,
        requested_by: Some(RequestedBy {
            kind: "hermes".into(),
            user_id: "user_e2e".into(),
        }),
        reason: "集成测试".to_string(),
        approval_token: token,
    }
}

fn call(addr: SocketAddr, request: &ToolRequest) -> Value {
    let body = serde_json::to_value(request).unwrap();
    let resp = http_client::post_json(addr, "/v1/tools/execute", &body, TIMEOUT)
        .expect("HTTP 调用失败");
    resp.body
}

fn status_of(body: &Value) -> &str {
    body["status"].as_str().unwrap_or("")
}

// ---------- 测试用例 ----------

#[test]
fn tools_listing_returns_eleven_skills() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let resp = http_client::get(addr, "/v1/tools", TIMEOUT).unwrap();
    let tools = resp.body["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 11);
    assert!(tools.iter().any(|t| t["name"] == json!("android.device_info")));
}

#[test]
fn successful_call_returns_result_and_audit_id() {
    let (addr, audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(addr, &make_request("android.device_info", json!({}), "ok_1", None));
    assert_eq!(status_of(&body), "success");
    assert!(body["result"]["model"].is_string());
    assert!(body["audit_id"].as_str().unwrap().starts_with("audit_"));
    assert!(body["duration_ms"].as_u64().unwrap() < 10_000);

    let records = AuditLogger::read_all(&audit).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status, "success");
    assert_eq!(records[0].arguments_hash.len(), 64);
}

#[test]
fn duplicate_request_id_is_rejected() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let first = call(addr, &make_request("android.device_info", json!({}), "dup_1", None));
    assert_eq!(status_of(&first), "success");
    let second = call(addr, &make_request("android.device_info", json!({}), "dup_1", None));
    assert_eq!(status_of(&second), "DUPLICATE_REQUEST");
}

#[test]
fn invalid_arguments_rejected() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(addr, &make_request("android.app.launch", json!({}), "inv_1", None));
    assert_eq!(status_of(&body), "INVALID_ARGUMENT");
    assert!(body["message"].as_str().unwrap().contains("package_name"));
}

#[test]
fn sensitive_path_blocked_by_policy() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(
        addr,
        &make_request("android.file.list", json!({ "path": "/system/etc" }), "path_1", None),
    );
    assert_eq!(status_of(&body), "POLICY_BLOCKED");
}

#[test]
fn sandboxed_path_is_allowed() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(
        addr,
        &make_request("android.file.list", json!({ "path": "/sdcard/Download" }), "path_2", None),
    );
    assert_eq!(status_of(&body), "success");
    assert!(body["result"]["entries"].is_array());
}

#[test]
fn unknown_tool_blocked() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(addr, &make_request("android.mystery.tool", json!({}), "unk_1", None));
    assert_eq!(status_of(&body), "POLICY_BLOCKED");
}

#[test]
fn arbitrary_shell_denied() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    let body = call(addr, &make_request("shell.exec", json!({}), "shell_1", None));
    assert_eq!(status_of(&body), "POLICY_BLOCKED");
}

#[test]
fn device_offline_when_agent_unreachable() {
    // 指向几乎不可能监听的端口 1
    let dead: SocketAddr = "127.0.0.1:1".parse().unwrap();
    let (addr, _audit) = spawn_gateway(default_policy(), dead, 1000);
    let body = call(addr, &make_request("android.device_info", json!({}), "off_1", None));
    assert_eq!(status_of(&body), "DEVICE_OFFLINE");
}

#[test]
fn approval_flow_then_success() {
    let policy = PolicyConfig::from_yaml(
        r#"
version: 1
defaults:
  allow_unknown_tools: false
  max_execution_time_ms: 15000
  require_audit: true
  require_idempotency_key: true
rules:
  - name: approve-screen-capture
    match:
      tool: android.screen.capture
    action: require_approval
  - name: allow-readonly-device
    match:
      risk_level: low
      read_only: true
    action: allow
"#,
    )
    .unwrap();
    let (addr, _audit) = spawn_gateway(policy, spawn_agent(), 1000);

    // 无令牌 → 需要审批
    let denied = call(addr, &make_request("android.screen.capture", json!({}), "apv_1", None));
    assert_eq!(status_of(&denied), "APPROVAL_REQUIRED");

    // 签发令牌
    let ticket_body = json!({ "request_id": "apv_1", "tool": "android.screen.capture", "reason": "测试审批" });
    let ticket = http_client::post_json(addr, "/v1/admin/approve", &ticket_body, TIMEOUT).unwrap();
    let token = ticket.body["token"].as_str().unwrap().to_string();
    assert!(!token.is_empty());

    // 带令牌 → 成功（新 request_id，避免幂等命中前次失败）
    let approved = call(
        addr,
        &make_request("android.screen.capture", json!({}), "apv_2", Some(token)),
    );
    assert_eq!(status_of(&approved), "success");
    assert_eq!(approved["result"]["format"], json!("png"));
}

#[test]
fn approval_token_is_single_use() {
    let policy = PolicyConfig::from_yaml(
        r#"
version: 1
defaults:
  allow_unknown_tools: false
  max_execution_time_ms: 15000
  require_audit: true
  require_idempotency_key: true
rules:
  - name: approve-screen-capture
    match:
      tool: android.screen.capture
    action: require_approval
"#,
    )
    .unwrap();
    let (addr, _audit) = spawn_gateway(policy, spawn_agent(), 1000);
    let ticket_body = json!({ "request_id": "once_1", "tool": "android.screen.capture", "reason": "" });
    let ticket = http_client::post_json(addr, "/v1/admin/approve", &ticket_body, TIMEOUT).unwrap();
    let token = ticket.body["token"].as_str().unwrap().to_string();

    let first = call(addr, &make_request("android.screen.capture", json!({}), "once_a", Some(token.clone())));
    assert_eq!(status_of(&first), "success");
    // 同一令牌再次使用应失败
    let second = call(addr, &make_request("android.screen.capture", json!({}), "once_b", Some(token)));
    assert_eq!(status_of(&second), "APPROVAL_REQUIRED");
}

#[test]
fn emergency_stop_blocks_write_then_releases() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);

    // 启用紧急停止
    let on = http_client::post_json(addr, "/v1/admin/emergency", &json!({ "engaged": true }), TIMEOUT).unwrap();
    assert_eq!(on.body["engaged"], json!(true));

    // 写操作（app.launch read_only=false）被拦截
    let blocked = call(
        addr,
        &make_request("android.app.launch", json!({ "package_name": "com.x" }), "es_1", None),
    );
    assert_eq!(status_of(&blocked), "FORBIDDEN");

    // 只读操作仍放行
    let readonly = call(addr, &make_request("android.device_info", json!({}), "es_2", None));
    assert_eq!(status_of(&readonly), "success");

    // 解除后写操作恢复
    let off = http_client::post_json(addr, "/v1/admin/emergency", &json!({ "engaged": false }), TIMEOUT).unwrap();
    assert_eq!(off.body["engaged"], json!(false));
    let resumed = call(
        addr,
        &make_request("android.app.launch", json!({ "package_name": "com.x" }), "es_3", None),
    );
    assert_eq!(status_of(&resumed), "success");
}

#[test]
fn rate_limit_blocks_excess_calls() {
    let (addr, _audit) = spawn_gateway(default_policy(), spawn_agent(), 2);
    let a = call(addr, &make_request("android.device_info", json!({}), "rl_1", None));
    let b = call(addr, &make_request("android.device_info", json!({}), "rl_2", None));
    let c = call(addr, &make_request("android.device_info", json!({}), "rl_3", None));
    assert_eq!(status_of(&a), "success");
    assert_eq!(status_of(&b), "success");
    assert_eq!(status_of(&c), "FORBIDDEN");
    assert!(c["message"].as_str().unwrap().contains("限频"));
}

#[test]
fn execution_timeout_when_agent_silent() {
    let policy = PolicyConfig::from_yaml(
        r#"
version: 1
defaults:
  allow_unknown_tools: false
  max_execution_time_ms: 300
  require_audit: true
  require_idempotency_key: true
rules:
  - name: allow-readonly-device
    match:
      risk_level: low
      read_only: true
    action: allow
"#,
    )
    .unwrap();
    let blackhole = spawn_blackhole();
    let (addr, _audit) = spawn_gateway(policy, blackhole, 1000);
    let body = call(addr, &make_request("android.device_info", json!({}), "to_1", None));
    assert_eq!(status_of(&body), "TIMEOUT");
}

#[test]
fn audit_log_accumulates_all_decisions() {
    let (addr, audit) = spawn_gateway(default_policy(), spawn_agent(), 1000);
    call(addr, &make_request("android.device_info", json!({}), "au_1", None));
    call(addr, &make_request("android.file.list", json!({ "path": "/system" }), "au_2", None));
    call(addr, &make_request("android.app.launch", json!({}), "au_3", None));

    let records = AuditLogger::read_all(&audit).unwrap();
    assert_eq!(records.len(), 3);
    let statuses: Vec<&str> = records.iter().map(|r| r.status.as_str()).collect();
    assert!(statuses.contains(&"success"));
    assert!(statuses.contains(&"POLICY_BLOCKED"));
    assert!(statuses.contains(&"INVALID_ARGUMENT"));
    // 每条都含合法 arguments_hash
    assert!(records.iter().all(|r| r.arguments_hash.len() == 64));
}
