//! JSONL 审计日志：每次调用一行，含 arguments_hash（SHA-256 十六进制）。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// 审计记录。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub audit_id: String,
    pub request_id: String,
    pub device_id: String,
    pub tool: String,
    pub status: String,
    pub arguments_hash: String,
    pub timestamp: String,
    #[serde(default)]
    pub reason: String,
}

/// 参数规范化后的 SHA-256 哈希，保证相同参数产生相同摘要。
pub fn arguments_hash(arguments: &Value) -> String {
    let canonical = serde_json::to_string(arguments).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 追加式 JSONL 审计写入器。
pub struct AuditLogger {
    path: PathBuf,
    seq: u64,
}

impl AuditLogger {
    pub fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // 预创建文件，确保目录可写
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(AuditLogger { path, seq: 0 })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一条审计记录并返回它（audit_id 自增生成）。
    pub fn log(
        &mut self,
        request_id: &str,
        device_id: &str,
        tool: &str,
        status: &str,
        arguments: &Value,
        reason: &str,
    ) -> std::io::Result<AuditRecord> {
        self.seq += 1;
        let record = AuditRecord {
            audit_id: format!("audit_{:06}", self.seq),
            request_id: request_id.to_string(),
            device_id: device_id.to_string(),
            tool: tool.to_string(),
            status: status.to_string(),
            arguments_hash: arguments_hash(arguments),
            timestamp: now_iso8601(),
            reason: reason.to_string(),
        };
        let file = File::options().append(true).open(&self.path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(record)
    }

    /// 读取全部审计记录（用于测试与审计检查）。
    pub fn read_all(path: impl AsRef<Path>) -> std::io::Result<Vec<AuditRecord>> {
        let text = std::fs::read_to_string(path)?;
        Ok(text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect())
    }
}

/// 本地时区无关的粗略 ISO-8601 时间戳（秒级，UTC）。
pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 简化：输出 epoch 秒对应的 ISO 日期需要日历计算，这里用固定格式占位日期+epoch。
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let rem = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant 的 civil_from_days 算法（自 1970-01-01 起的天数 → 年月日）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hash_is_stable_and_sensitive() {
        let a = arguments_hash(&json!({ "package_name": "com.x" }));
        let b = arguments_hash(&json!({ "package_name": "com.x" }));
        let c = arguments_hash(&json!({ "package_name": "com.y" }));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn logger_appends_jsonl_and_read_back() {
        let dir = std::env::temp_dir().join(format!("hermes_audit_{}", std::process::id()));
        let path = dir.join("audit.jsonl");
        let _ = std::fs::remove_file(&path);
        let mut logger = AuditLogger::new(&path).unwrap();
        let r1 = logger
            .log("req_1", "dev_1", "android.device_info", "success", &json!({}), "测试")
            .unwrap();
        let r2 = logger
            .log("req_2", "dev_1", "android.file.list", "POLICY_BLOCKED", &json!({"path":"/system"}), "")
            .unwrap();
        assert_eq!(r1.audit_id, "audit_000001");
        assert_eq!(r2.audit_id, "audit_000002");
        let records = AuditLogger::read_all(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].status, "POLICY_BLOCKED");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamp_format_is_iso8601() {
        let ts = now_iso8601();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
    }
}
