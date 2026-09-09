//! Tool Registry：扫描 skills/ 目录，加载每个技能的 manifest 与输入 Schema。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hermes_core::schema::ObjectSchema;
use serde::Deserialize;

/// 技能清单（manifest.yaml）。
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub read_only: Option<bool>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub idempotent: bool,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub input_schema: Option<String>,
}

/// 已注册工具：清单 + 解析后的输入 Schema。
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    pub manifest: Manifest,
    pub input_schema: ObjectSchema,
}

/// 工具注册表。
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, RegisteredTool>,
}

impl ToolRegistry {
    /// 从 skills/ 根目录加载全部技能。
    ///
    /// 每个技能目录形如 `skills/<tool>/`，含 `manifest.yaml` 与
    /// `manifest.input_schema` 指向的 schema 文件（相对技能目录）。
    pub fn load_from_dir(skills_dir: &Path) -> Result<Self, String> {
        let mut registry = ToolRegistry::default();
        let entries = std::fs::read_dir(skills_dir)
            .map_err(|e| format!("无法读取 skills 目录 {}: {e}", skills_dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            match Self::load_skill(&path) {
                Ok(tool) => {
                    registry.tools.insert(tool.manifest.name.clone(), tool);
                }
                Err(e) => return Err(format!("加载技能 {} 失败: {e}", path.display())),
            }
        }
        if registry.tools.is_empty() {
            return Err(format!("skills 目录 {} 下没有有效技能", skills_dir.display()));
        }
        Ok(registry)
    }

    fn load_skill(skill_dir: &Path) -> Result<RegisteredTool, String> {
        let manifest_path = skill_dir.join("manifest.yaml");
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("读取 manifest.yaml 失败: {e}"))?;
        let manifest: Manifest =
            serde_yaml::from_str(&manifest_text).map_err(|e| format!("解析 manifest 失败: {e}"))?;

        let schema_rel = manifest.input_schema.clone().unwrap_or_default();
        let schema_path = if schema_rel.is_empty() {
            skill_dir.join("schema").join("input.json")
        } else {
            skill_dir.join(&schema_rel)
        };
        let schema_text = std::fs::read_to_string(&schema_path)
            .map_err(|e| format!("读取 {} 失败: {e}", schema_rel))?;
        let input_schema = ObjectSchema::from_json(&schema_text)?;

        Ok(RegisteredTool {
            manifest,
            input_schema,
        })
    }

    /// 查询工具。
    pub fn get(&self, tool: &str) -> Option<&RegisteredTool> {
        self.tools.get(tool)
    }

    /// 工具是否已注册。
    pub fn contains(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }

    /// 全部工具名（有序）。
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 工具数量。
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// 定位仓库根目录下的 skills/（从当前工作目录或可执行文件向上查找）。
    pub fn locate_skills_dir(start: &Path) -> Option<PathBuf> {
        let mut dir = start.to_path_buf();
        loop {
            let candidate = dir.join("skills");
            if candidate.is_dir() {
                return Some(candidate);
            }
            if !dir.pop() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn loads_all_bundled_skills() {
        let skills = repo_root().join("skills");
        let registry = ToolRegistry::load_from_dir(&skills).expect("应能加载 skills 目录");
        assert_eq!(registry.len(), 11, "实际: {:?}", registry.names());
        assert!(registry.contains("android.device_info"));
        assert!(registry.contains("android.file.list"));
    }

    #[test]
    fn manifest_fields_are_parsed() {
        let registry = ToolRegistry::load_from_dir(&repo_root().join("skills")).unwrap();
        let launch = registry.get("android.app.launch").unwrap();
        assert_eq!(launch.manifest.risk_level.as_deref(), Some("low"));
        assert_eq!(launch.manifest.read_only, Some(false));
        assert!(launch.manifest.permissions.contains(&"app.launch".to_string()));
    }

    #[test]
    fn input_schema_validates_arguments() {
        let registry = ToolRegistry::load_from_dir(&repo_root().join("skills")).unwrap();
        let launch = registry.get("android.app.launch").unwrap();
        let errors = launch
            .input_schema
            .validate(&serde_json::json!({ "package_name": "com.android.settings" }));
        assert!(errors.is_empty(), "{errors:?}");
        let errors = launch.input_schema.validate(&serde_json::json!({}));
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn locate_skills_dir_walks_up() {
        let found = ToolRegistry::locate_skills_dir(&repo_root().join("crates").join("gateway"))
            .expect("应向上定位到 skills");
        assert!(found.ends_with("skills"));
    }
}
