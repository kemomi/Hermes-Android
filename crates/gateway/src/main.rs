//! gateway 二进制入口：加载策略与技能，启动回环 HTTP 服务。
//!
//! 用法：
//!   gateway [--http 127.0.0.1:8787] [--agent 127.0.0.1:7836]
//!           [--skills <dir>] [--policy <file>] [--audit <file>]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gateway::http_server::{self, HttpServerConfig};
use gateway::policy::PolicyConfig;
use gateway::router::AgentEndpoint;
use gateway::Gateway;

struct Args {
    http: SocketAddr,
    agent: SocketAddr,
    skills: PathBuf,
    policy: PathBuf,
    audit: PathBuf,
}

fn parse_args() -> Args {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut args = Args {
        http: "127.0.0.1:8787".parse().unwrap(),
        agent: "127.0.0.1:7836".parse().unwrap(),
        skills: cwd.join("skills"),
        policy: cwd.join("policies").join("default.yaml"),
        audit: cwd.join("run").join("audit.jsonl"),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let value = it.next();
        match flag.as_str() {
            "--http" => {
                if let Some(v) = value {
                    args.http = v.parse().expect("--http 需为 host:port");
                }
            }
            "--agent" => {
                if let Some(v) = value {
                    args.agent = v.parse().expect("--agent 需为 host:port");
                }
            }
            "--skills" => {
                if let Some(v) = value {
                    args.skills = PathBuf::from(v);
                }
            }
            "--policy" => {
                if let Some(v) = value {
                    args.policy = PathBuf::from(v);
                }
            }
            "--audit" => {
                if let Some(v) = value {
                    args.audit = PathBuf::from(v);
                }
            }
            other => eprintln!("忽略未知参数 {other}"),
        }
    }
    args
}

fn main() {
    let args = parse_args();

    // 强制回环监听（蓝图禁止 0.0.0.0）
    if !args.http.ip().is_loopback() {
        eprintln!("安全策略：Gateway 仅允许监听回环地址，收到 {}", args.http);
        std::process::exit(2);
    }

    let policy_text = std::fs::read_to_string(&args.policy)
        .unwrap_or_else(|e| panic!("读取策略文件 {} 失败: {e}", args.policy.display()));
    let policy =
        PolicyConfig::from_yaml(&policy_text).unwrap_or_else(|e| panic!("解析策略失败: {e}"));

    let agent = AgentEndpoint {
        addr: args.agent,
        connect_timeout: Duration::from_secs(2),
    };

    let gw = Gateway::new(
        &args.skills,
        policy,
        agent,
        &args.audit,
        600,
        Duration::from_secs(60),
    )
    .unwrap_or_else(|e| panic!("初始化 Gateway 失败: {e}"));

    eprintln!(
        "已加载 {} 个技能，审计写入 {}",
        gw.registry.len(),
        args.audit.display()
    );

    let gateway = Arc::new(gw);
    http_server::serve(HttpServerConfig { bind: args.http }, gateway)
        .unwrap_or_else(|e| panic!("HTTP 服务启动失败: {e}"));
}
