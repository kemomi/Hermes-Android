//! hermes-cli：通过 Gateway 的 HTTP 接口列出工具、发起调用、签发审批与紧急停止。

use std::net::SocketAddr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use hermes_core::http_client;
use hermes_core::protocol::{RequestedBy, ToolRequest};
use serde_json::{json, Value};

/// Hermes 命令行客户端。
#[derive(Parser, Debug)]
#[command(name = "hermes-cli", version, about = "Hermes-Android 控制面命令行")]
struct Cli {
    /// Gateway 地址（host:port）。
    #[arg(long, default_value = "127.0.0.1:8787", global = true)]
    gateway: String,

    /// 请求超时（秒）。
    #[arg(long, default_value_t = 20, global = true)]
    timeout: u64,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 列出 Gateway 已注册的工具。
    Tools,
    /// 调用一个工具。
    Call {
        /// 工具名，如 android.device_info。
        tool: String,
        /// JSON 参数，如 '{"package_name":"com.android.settings"}'。
        #[arg(long, default_value = "{}")]
        args: String,
        /// 请求 ID（缺省自动生成）。
        #[arg(long)]
        request_id: Option<String>,
        /// 设备 ID。
        #[arg(long, default_value = "device_001")]
        device_id: String,
        /// 调用原因（写入审计）。
        #[arg(long, default_value = "")]
        reason: String,
        /// 人工审批令牌。
        #[arg(long)]
        approval_token: Option<String>,
    },
    /// 为待审批请求签发一次性令牌。
    Approve {
        /// 关联的 request_id。
        #[arg(long)]
        request_id: String,
        /// 工具名。
        #[arg(long)]
        tool: String,
        /// 审批原因。
        #[arg(long, default_value = "")]
        reason: String,
    },
    /// 紧急停止开关。
    #[command(name = "stop")]
    Emergency {
        /// 传入则解除；缺省为启用紧急停止。
        #[arg(long)]
        release: bool,
    },
}

fn addr_of(cli: &Cli) -> SocketAddr {
    cli.gateway
        .parse()
        .unwrap_or_else(|_| panic!("非法 Gateway 地址 {}", cli.gateway))
}

fn main() {
    let cli = Cli::parse();
    let addr = addr_of(&cli);
    let timeout = Duration::from_secs(cli.timeout);

    let result: Result<Value, String> = match &cli.command {
        Command::Tools => http_client::get(addr, "/v1/tools", timeout)
            .map(|r| r.body)
            .map_err(|e| e.to_string()),

        Command::Call {
            tool,
            args,
            request_id,
            device_id,
            reason,
            approval_token,
        } => {
            let arguments: Value =
                serde_json::from_str(args).map_err(|e| format!("--args 不是合法 JSON: {e}"))?;
            let request = ToolRequest {
                request_id: request_id
                    .clone()
                    .unwrap_or_else(|| format!("req_{}", std::process::id())),
                session_id: format!("cli_{}", std::process::id()),
                device_id: device_id.clone(),
                tool: tool.clone(),
                arguments,
                requested_by: Some(RequestedBy {
                    kind: "hermes-cli".into(),
                    user_id: "local".into(),
                }),
                reason: reason.clone(),
                approval_token: approval_token.clone(),
            };
            let body = serde_json::to_value(&request).map_err(|e| e.to_string())?;
            http_client::post_json(addr, "/v1/tools/execute", &body, timeout)
                .map(|r| r.body)
                .map_err(|e| e.to_string())
        }

        Command::Approve {
            request_id,
            tool,
            reason,
        } => {
            let body = json!({
                "request_id": request_id,
                "tool": tool,
                "reason": reason,
            });
            http_client::post_json(addr, "/v1/admin/approve", &body, timeout)
                .map(|r| r.body)
                .map_err(|e| e.to_string())
        }

        Command::Emergency { release } => {
            let body = json!({ "engaged": !release });
            http_client::post_json(addr, "/v1/admin/emergency", &body, timeout)
                .map(|r| r.body)
                .map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default());
        }
        Err(e) => {
            eprintln!("请求失败: {e}");
            std::process::exit(1);
        }
    }
}
