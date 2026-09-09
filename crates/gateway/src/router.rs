//! 执行路由：通过回环 TCP 将已校验的请求转发给 Android Agent，带执行超时。

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use hermes_core::protocol::{Status, ToolRequest, ToolResponse};

/// 与单个 Agent 的连接配置。
#[derive(Debug, Clone)]
pub struct AgentEndpoint {
    pub addr: SocketAddr,
    pub connect_timeout: Duration,
}

/// 执行路由结果：成功转发得到响应，或连接/超时错误。
#[derive(Debug)]
pub enum RouteError {
    /// Agent 不可达 / 离线。
    Offline(String),
    /// 执行超时。
    Timeout,
    /// 协议层错误（响应无法解析等）。
    Protocol(String),
}

/// 将请求转发给 Agent，读取一行 JSON 响应。
pub fn forward(endpoint: &AgentEndpoint, request: &ToolRequest, exec_timeout: Duration) -> Result<ToolResponse, RouteError> {
    let stream = TcpStream::connect_timeout(&endpoint.addr, endpoint.connect_timeout)
        .map_err(|e| RouteError::Offline(format!("连接 Agent 失败: {e}")))?;
    stream
        .set_read_timeout(Some(exec_timeout))
        .map_err(|e| RouteError::Offline(e.to_string()))?;
    stream
        .set_write_timeout(Some(exec_timeout))
        .map_err(|e| RouteError::Offline(e.to_string()))?;

    let mut writer = stream.try_clone().map_err(|e| RouteError::Offline(e.to_string()))?;
    let mut payload = serde_json::to_string(request)
        .map_err(|e| RouteError::Protocol(format!("请求序列化失败: {e}")))?;
    payload.push('\n');
    writer
        .write_all(payload.as_bytes())
        .map_err(|e| RouteError::Offline(format!("写入 Agent 失败: {e}")))?;
    writer
        .flush()
        .map_err(|e| RouteError::Offline(format!("刷新 Agent 失败: {e}")))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Err(RouteError::Offline("Agent 提前关闭连接".into())),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
            Err(RouteError::Timeout)
        }
        Err(e) => Err(RouteError::Offline(format!("读取 Agent 响应失败: {e}"))),
        Ok(_) => {
            serde_json::from_str::<ToolResponse>(line.trim())
                .map_err(|e| RouteError::Protocol(format!("响应解析失败: {e}")))
        }
    }
}

/// 把路由错误映射为标准化响应。
pub fn error_to_response(request: &ToolRequest, err: RouteError) -> ToolResponse {
    let (status, message) = match err {
        RouteError::Offline(m) => (Status::DeviceOffline, m),
        RouteError::Timeout => (Status::Timeout, "执行超时".to_string()),
        RouteError::Protocol(m) => (Status::ExecutionFailed, m),
    };
    ToolResponse::failure(&request.request_id, &request.tool, status, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_request() -> ToolRequest {
        ToolRequest {
            request_id: "req_route".into(),
            session_id: String::new(),
            device_id: "device_001".into(),
            tool: "android.device_info".into(),
            arguments: json!({}),
            requested_by: None,
            reason: String::new(),
            approval_token: None,
        }
    }

    #[test]
    fn offline_when_no_agent_listening() {
        // 端口 1 几乎不可能有监听
        let endpoint = AgentEndpoint {
            addr: "127.0.0.1:1".parse().unwrap(),
            connect_timeout: Duration::from_millis(500),
        };
        let err = forward(&endpoint, &sample_request(), Duration::from_secs(2)).unwrap_err();
        let resp = error_to_response(&sample_request(), err);
        assert_eq!(resp.status, Status::DeviceOffline);
        assert_eq!(resp.request_id, "req_route");
    }
}
