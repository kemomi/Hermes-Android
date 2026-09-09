//! 最小 HTTP/1.1 客户端：基于 std TcpStream，仅支持 Content-Length 响应。
//!
//! 供 CLI 与集成测试调用 Gateway，不引入 tokio/reqwest。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::Value;

/// HTTP 响应。
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

/// 客户端错误。
#[derive(Debug)]
pub enum ClientError {
    Connect(String),
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Connect(m) => write!(f, "连接失败: {m}"),
            ClientError::Io(m) => write!(f, "IO 错误: {m}"),
            ClientError::Parse(m) => write!(f, "响应解析失败: {m}"),
        }
    }
}

/// 发起一次 HTTP 请求（method + path + 可选 JSON 体）。
pub fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&Value>,
    timeout: Duration,
) -> Result<HttpResponse, ClientError> {
    let stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| ClientError::Connect(e.to_string()))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| ClientError::Io(e.to_string()))?;
    let mut stream = stream;

    let body_bytes = match body {
        Some(v) => serde_json::to_vec(v).map_err(|e| ClientError::Parse(e.to_string()))?,
        None => Vec::new(),
    };
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body_bytes.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|e| ClientError::Io(e.to_string()))?;
    if !body_bytes.is_empty() {
        stream
            .write_all(&body_bytes)
            .map_err(|e| ClientError::Io(e.to_string()))?;
    }
    stream.flush().map_err(|e| ClientError::Io(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|e| ClientError::Io(e.to_string()))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| ClientError::Parse(format!("非法状态行: {status_line}")))?;

    let mut content_length = None;
    let mut chunked = false;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).map_err(|e| ClientError::Io(e.to_string()))? == 0 {
            break;
        }
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            if name == "content-length" {
                content_length = value.trim().parse().ok();
            } else if name == "transfer-encoding" && value.to_lowercase().contains("chunked") {
                chunked = true;
            }
        }
    }

    let mut raw = Vec::new();
    if let Some(len) = content_length {
        let mut buf = vec![0u8; len];
        reader
            .read_exact(&mut buf)
            .map_err(|e| ClientError::Io(e.to_string()))?;
        raw = buf;
    } else if chunked {
        raw = read_chunked(&mut reader)?;
    } else {
        reader
            .read_to_end(&mut raw)
            .map_err(|e| ClientError::Io(e.to_string()))?;
    }

    let text = String::from_utf8_lossy(&raw);
    let body = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).map_err(|e| ClientError::Parse(format!("{e}: {text}")))?
    };
    Ok(HttpResponse { status, body })
}

fn read_chunked<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, ClientError> {
    let mut out = Vec::new();
    loop {
        let mut size_line = String::new();
        reader
            .read_line(&mut size_line)
            .map_err(|e| ClientError::Io(e.to_string()))?;
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|e| ClientError::Parse(e.to_string()))?;
        if size == 0 {
            break;
        }
        let mut buf = vec![0u8; size];
        reader
            .read_exact(&mut buf)
            .map_err(|e| ClientError::Io(e.to_string()))?;
        out.extend_from_slice(&buf);
        // 读取块尾 CRLF
        let mut crlf = String::new();
        let _ = reader.read_line(&mut crlf);
    }
    Ok(out)
}

/// 便捷方法：POST JSON。
pub fn post_json(addr: SocketAddr, path: &str, body: &Value, timeout: Duration) -> Result<HttpResponse, ClientError> {
    request(addr, "POST", path, Some(body), timeout)
}

/// 便捷方法：GET。
pub fn get(addr: SocketAddr, path: &str, timeout: Duration) -> Result<HttpResponse, ClientError> {
    request(addr, "GET", path, None, timeout)
}
