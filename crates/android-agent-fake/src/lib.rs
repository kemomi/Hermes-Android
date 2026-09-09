//! Phase 0 模拟 Android Agent 库：TCP 行协议服务与模拟工具实现。

pub mod sandbox;
pub mod tools;

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use hermes_core::protocol::{Status, ToolRequest, ToolResponse};
use hermes_core::schema::ObjectSchema;

/// 内置参数 Schema（与 skills/ 下 input.json 保持一致的最小子集）。
pub fn schema_for(tool: &str) -> Option<ObjectSchema> {
    let json = match tool {
        tools::names::APP_LAUNCH => {
            r#"{"type":"object","properties":{"package_name":{"type":"string","pattern":"^\\S+$"}},"required":["package_name"],"additionalProperties":false}"#
        }
        tools::names::FILE_LIST => {
            r#"{"type":"object","properties":{"path":{"type":"string","pattern":"^/"}},"required":["path"],"additionalProperties":false}"#
        }
        _ => r#"{"type":"object","properties":{},"additionalProperties":false}"#,
    };
    ObjectSchema::from_json(json).ok()
}

/// 处理单条请求：Schema 校验后执行模拟工具。
pub fn handle_request(request: &ToolRequest) -> ToolResponse {
    match schema_for(&request.tool) {
        Some(schema) => {
            let errors = schema.validate(&request.arguments);
            if errors.is_empty() {
                tools::execute(request)
            } else {
                ToolResponse::failure(
                    &request.request_id,
                    &request.tool,
                    Status::InvalidArgument,
                    errors.join("; "),
                )
            }
        }
        None => ToolResponse::failure(
            &request.request_id,
            &request.tool,
            Status::InvalidArgument,
            "无法加载参数 Schema",
        ),
    }
}

fn handle_connection(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let response = match serde_json::from_str::<ToolRequest>(line.trim()) {
            Ok(request) => handle_request(&request),
            Err(e) => ToolResponse {
                request_id: String::new(),
                status: Status::InvalidArgument,
                tool: String::new(),
                result: None,
                audit_id: None,
                duration_ms: 0,
                message: Some(format!("请求 JSON 解析失败: {e}")),
            },
        };
        let mut out = serde_json::to_string(&response).unwrap_or_default();
        out.push('\n');
        if writer.write_all(out.as_bytes()).is_err() || writer.flush().is_err() {
            return;
        }
    }
}

/// 在回环地址启动 Agent 服务（端口 0 表示自动分配），返回实际监听地址。
pub fn serve(port: u16) -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let addr = listener.local_addr()?;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    std::thread::spawn(move || handle_connection(s));
                }
                Err(e) => eprintln!("accept 失败: {e}"),
            }
        }
    });
    Ok(addr)
}
