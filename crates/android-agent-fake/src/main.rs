//! android-agent-fake 二进制入口。

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(7836);
    let addr = android_agent_fake::serve(port).expect("绑定回环端口失败");
    eprintln!("fake android agent listening on {}", addr);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
