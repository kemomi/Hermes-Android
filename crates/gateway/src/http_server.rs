//! 最小 HTTP/1.1 服务：仅支持 Content-Length 请求体，暴露 Gateway 与审批/紧急停止接口。
//!
//! 路由：
//! - POST /v1/tools/execute   —— 统一工具调用
//! - POST /v1/admin/approve   —— 为待审批请求签发令牌
//! - POST /v1/admin/emergency —— {"engaged": true|false} 紧急停止开关
//! - GET  /v1/tools           —— 列出已注册工具

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::handler::Gateway;

/// 服务监听配置。
#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub bind: SocketAddr,
}

/// 解析后的 HTTP 请求。
struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<HttpRequest>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(None);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = String::new();
    if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)?;
        body = String::from_utf8_lossy(&buf).to_string();
    }
    Ok(Some(HttpRequest { method, path, body }))
}

fn write_response(stream: &mut TcpStream, status: u16, body: &Value) -> std::io::Result<()> {
    let payload = serde_json::to_vec(body).unwrap_or_default();
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

fn route(gw: &Gateway, req: HttpRequest) -> (u16, Value) {
    let path = req.path.split('?').next().unwrap_or("").to_string();
    match (req.method.as_str(), path.as_str()) {
        ("GET", "/v1/tools") => {
            let tools: Vec<Value> = gw
                .registry
                .names()
                .iter()
                .map(|name| {
                    let m = &gw.registry.get(name).unwrap().manifest;
                    json!({
                        "name": name,
                        "description": m.description,
                        "risk_level": m.risk_level,
                        "read_only": m.read_only,
                        "requires_approval": m.requires_approval,
                    })
                })
                .collect();
            (200, json!({ "tools": tools }))
        }
        ("POST", "/v1/tools/execute") => {
            let request = match serde_json::from_str(&req.body) {
                Ok(r) => r,
                Err(e) => return (400, json!({ "status": "INVALID_ARGUMENT", "message": e.to_string() })),
            };
            let response = gw.handle(&request);
            (200, serde_json::to_value(response).unwrap_or_else(|_| json!({})))
        }
        ("POST", "/v1/admin/approve") => {
            #[derive(Deserialize)]
            struct ApproveReq {
                request_id: String,
                tool: String,
                #[serde(default)]
                reason: String,
            }
            match serde_json::from_str::<ApproveReq>(&req.body) {
                Ok(a) => {
                    let ticket = gw.issue_approval(&a.request_id, &a.tool, &a.reason);
                    (200, serde_json::to_value(ticket).unwrap_or_else(|_| json!({})))
                }
                Err(e) => (400, json!({ "status": "INVALID_ARGUMENT", "message": e.to_string() })),
            }
        }
        ("POST", "/v1/admin/emergency") => {
            #[derive(Deserialize)]
            struct EmergencyReq {
                engaged: bool,
            }
            match serde_json::from_str::<EmergencyReq>(&req.body) {
                Ok(e) => {
                    if e.engaged {
                        gw.emergency_stop.engage();
                    } else {
                        gw.emergency_stop.release();
                    }
                    (200, json!({ "engaged": gw.emergency_stop.is_engaged() }))
                }
                Err(e) => (400, json!({ "status": "INVALID_ARGUMENT", "message": e.to_string() })),
            }
        }
        (_, p) if p.starts_with("/v1/") => (404, json!({ "status": "NOT_FOUND", "path": p })),
        _ => (405, json!({ "status": "METHOD_NOT_ALLOWED" })),
    }
}

/// 启动 HTTP 服务（阻塞）。返回实际绑定地址前会先 bind。
pub fn serve(config: HttpServerConfig, gateway: Arc<Gateway>) -> std::io::Result<()> {
    let listener = TcpListener::bind(config.bind)?;
    let addr = listener.local_addr()?;
    eprintln!("gateway http listening on http://{addr}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let gw = Arc::clone(&gateway);
                std::thread::spawn(move || {
                    let outcome = read_request(&mut s)
                        .map(|opt| opt.map(|req| route(&gw, req)))
                        .unwrap_or(None);
                    let (status, body) = outcome.unwrap_or((400, json!({ "status": "INVALID_ARGUMENT" })));
                    let _ = write_response(&mut s, status, &body);
                });
            }
            Err(e) => eprintln!("accept 失败: {e}"),
        }
    }
    Ok(())
}

/// 绑定并返回监听地址（用于测试拿到随机端口后再在后台线程 serve）。
pub fn bind(config: HttpServerConfig) -> std::io::Result<(SocketAddr, TcpListener)> {
    let listener = TcpListener::bind(config.bind)?;
    let addr = listener.local_addr()?;
    Ok((addr, listener))
}

/// 用已绑定的 listener 提供服务（阻塞），供测试在子线程中运行。
pub fn serve_with_listener(listener: TcpListener, gateway: Arc<Gateway>) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let gw = Arc::clone(&gateway);
                std::thread::spawn(move || {
                    let outcome = read_request(&mut s)
                        .map(|opt| opt.map(|req| route(&gw, req)))
                        .unwrap_or(None);
                    let (status, body) = outcome.unwrap_or((400, json!({ "status": "INVALID_ARGUMENT" })));
                    let _ = write_response(&mut s, status, &body);
                });
            }
            Err(e) => eprintln!("accept 失败: {e}"),
        }
    }
}
