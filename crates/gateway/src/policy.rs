//! 策略引擎：解析 policies/default.yaml，按规则顺序评估工具调用动作。

use serde::Deserialize;

/// 策略动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    RequireApproval,
    Deny,
}

/// tool 匹配：单个字符串或字符串列表。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ToolMatch {
    One(String),
    Many(Vec<String>),
}

impl ToolMatch {
    fn contains(&self, tool: &str) -> bool {
        match self {
            ToolMatch::One(t) => t == tool,
            ToolMatch::Many(ts) => ts.iter().any(|t| t == tool),
        }
    }
}

/// 规则匹配条件，多字段之间为 AND 关系。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuleMatch {
    #[serde(default)]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub read_only: Option<bool>,
    #[serde(default)]
    pub tool: Option<ToolMatch>,
    #[serde(default)]
    pub path_prefix: Option<Vec<String>>,
    #[serde(default)]
    pub namespace: Option<String>,
}

/// 单条策略规则。
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(rename = "match")]
    pub matcher: RuleMatch,
    pub action: Action,
}

/// 默认策略。
#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    pub allow_unknown_tools: bool,
    pub max_execution_time_ms: u64,
    pub require_audit: bool,
    pub require_idempotency_key: bool,
}

/// 完整策略配置。
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyConfig {
    pub version: u32,
    pub defaults: Defaults,
    pub rules: Vec<Rule>,
}

/// 策略评估主体：由 ToolRegistry 依据 manifest 与请求参数构造。
pub struct PolicySubject<'a> {
    pub tool: &'a str,
    pub risk_level: Option<&'a str>,
    pub read_only: Option<bool>,
    pub namespace: Option<&'a str>,
    pub path: Option<&'a str>,
    /// 工具是否在注册表中已知。
    pub known: bool,
    /// manifest 声明的 requires_approval（规则未命中时的回退依据）。
    pub requires_approval: bool,
}

/// 从工具名解析命名空间（去掉最后一段），如 kernel.write.temp → kernel.write。
pub fn namespace_of(tool: &str) -> Option<&str> {
    tool.rsplit_once('.').map(|(ns, _)| ns)
}

impl PolicyConfig {
    /// 从 YAML 文本解析策略。
    pub fn from_yaml(text: &str) -> Result<Self, String> {
        serde_yaml::from_str(text).map_err(|e| format!("策略解析失败: {e}"))
    }

    /// 评估动作：首个命中的规则生效；否则按已知性与默认审批回退。
    pub fn evaluate(&self, subject: &PolicySubject) -> Action {
        for rule in &self.rules {
            if self.rule_matches(rule, subject) {
                return rule.action;
            }
        }
        if !subject.known {
            return if self.defaults.allow_unknown_tools {
                Action::Allow
            } else {
                Action::Deny
            };
        }
        if subject.requires_approval {
            Action::RequireApproval
        } else {
            Action::Allow
        }
    }

    fn rule_matches(&self, rule: &Rule, s: &PolicySubject) -> bool {
        let m = &rule.matcher;
        if let Some(rl) = &m.risk_level {
            if s.risk_level != Some(rl.as_str()) {
                return false;
            }
        }
        if let Some(ro) = m.read_only {
            if s.read_only != Some(ro) {
                return false;
            }
        }
        if let Some(tm) = &m.tool {
            if !tm.contains(s.tool) {
                return false;
            }
        }
        if let Some(ns) = &m.namespace {
            if s.namespace != Some(ns.as_str()) {
                return false;
            }
        }
        if let Some(prefixes) = &m.path_prefix {
            let path = s.path.unwrap_or("");
            if !prefixes.iter().any(|p| path.starts_with(p.as_str())) {
                return false;
            }
        }
        true
    }
}

/// 加载仓库默认策略（测试与本地运行共用）。
pub fn default_policy() -> PolicyConfig {
    PolicyConfig::from_yaml(include_str!("../../../policies/default.yaml"))
        .expect("内置默认策略应可解析")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject<'a>(
        tool: &'a str,
        risk: Option<&'a str>,
        read_only: Option<bool>,
        path: Option<&'a str>,
        known: bool,
    ) -> PolicySubject<'a> {
        PolicySubject {
            tool,
            risk_level: risk,
            read_only,
            namespace: namespace_of(tool),
            path,
            known,
            requires_approval: false,
        }
    }

    #[test]
    fn parses_bundled_default_policy() {
        let cfg = default_policy();
        assert_eq!(cfg.version, 1);
        assert!(!cfg.defaults.allow_unknown_tools);
        assert_eq!(cfg.defaults.max_execution_time_ms, 15000);
        assert_eq!(cfg.rules.len(), 5);
    }

    #[test]
    fn allows_low_readonly_tool() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject("android.device_info", Some("low"), Some(true), None, true));
        assert_eq!(action, Action::Allow);
    }

    #[test]
    fn denies_arbitrary_shell_even_if_unknown() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject("shell.exec", None, None, None, false));
        assert_eq!(action, Action::Deny);
    }

    #[test]
    fn denies_sensitive_path() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject(
            "android.file.list",
            Some("medium"),
            Some(true),
            Some("/system/etc/hosts"),
            true,
        ));
        assert_eq!(action, Action::Deny);
    }

    #[test]
    fn allows_sandboxed_path_when_no_rule_matches() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject(
            "android.file.list",
            Some("medium"),
            Some(true),
            Some("/sdcard/Download/a.txt"),
            true,
        ));
        assert_eq!(action, Action::Allow);
    }

    #[test]
    fn requires_approval_for_file_delete() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject("android.file.delete", Some("high"), Some(false), None, true));
        assert_eq!(action, Action::RequireApproval);
    }

    #[test]
    fn requires_approval_for_kernel_write_namespace() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject(
            "kernel.write.temperature",
            Some("critical"),
            Some(false),
            None,
            true,
        ));
        assert_eq!(action, Action::RequireApproval);
    }

    #[test]
    fn denies_unknown_tool_by_default() {
        let cfg = default_policy();
        let action = cfg.evaluate(&subject("android.unknown.thing", None, None, None, false));
        assert_eq!(action, Action::Deny);
    }

    #[test]
    fn namespace_extraction() {
        assert_eq!(namespace_of("kernel.write.temp"), Some("kernel.write"));
        assert_eq!(namespace_of("android.app.launch"), Some("android.app"));
        assert_eq!(namespace_of("device_info"), None);
    }
}
