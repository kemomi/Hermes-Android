//! JSON Schema 最小子集校验：type / properties / pattern / required / additionalProperties。
//!
//! 仅覆盖蓝图 manifest 的 schema/input.json 所用关键字，避免引入重量级依赖。

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// 属性约束。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type", default)]
    pub ty: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
}

/// 对象级 Schema。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectSchema {
    #[serde(rename = "type", default)]
    pub ty: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, PropertySchema>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub additional_properties: Option<bool>,
}

impl ObjectSchema {
    /// 从 schema/input.json 文本解析；`additionalProperties` 用 serde 别名兼容。
    pub fn from_json(text: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "type", default)]
            ty: Option<String>,
            #[serde(default)]
            properties: Option<BTreeMap<String, PropertySchema>>,
            #[serde(default)]
            required: Option<Vec<String>>,
            #[serde(alias = "additionalProperties", default)]
            additional_properties: Option<bool>,
        }
        let raw: Raw = serde_json::from_str(text).map_err(|e| format!("schema 解析失败: {e}"))?;
        Ok(ObjectSchema {
            ty: raw.ty,
            properties: raw.properties.unwrap_or_default(),
            required: raw.required.unwrap_or_default(),
            additional_properties: raw.additional_properties,
        })
    }

    /// 校验参数对象，返回全部错误（空 Vec 表示通过）。
    pub fn validate(&self, args: &Value) -> Vec<String> {
        let mut errors = Vec::new();
        if self.ty.as_deref() == Some("object") && !args.is_object() {
            errors.push("参数必须是 object".to_string());
            return errors;
        }
        let obj: &Map<String, Value> = match args.as_object() {
            Some(o) => o,
            None => return errors,
        };
        for field in &self.required {
            if !obj.contains_key(field) {
                errors.push(format!("缺少必填参数 {field}"));
            }
        }
        if self.additional_properties == Some(false) {
            for key in obj.keys() {
                if !self.properties.contains_key(key) {
                    errors.push(format!("不允许的额外参数 {key}"));
                }
            }
        }
        for (key, value) in obj {
            let Some(prop) = self.properties.get(key) else { continue };
            match prop.ty.as_deref() {
                Some("string") => {
                    let s = match value.as_str() {
                        Some(s) => s,
                        None => {
                            errors.push(format!("参数 {key} 必须是 string"));
                            continue;
                        }
                    };
                    if let Some(pattern) = &prop.pattern {
                        match Regex::new(pattern) {
                            Ok(re) if re.is_match(s) => {}
                            Ok(_) => errors.push(format!("参数 {key} 不匹配 pattern {pattern}")),
                            Err(e) => errors.push(format!("参数 {key} 的 pattern 非法: {e}")),
                        }
                    }
                }
                Some("integer") => {
                    if !value.is_i64() && !value.is_u64() {
                        errors.push(format!("参数 {key} 必须是 integer"));
                    }
                }
                Some("boolean") => {
                    if !value.is_boolean() {
                        errors.push(format!("参数 {key} 必须是 boolean"));
                    }
                }
                Some(other) => errors.push(format!("参数 {key} 的类型 {other} 不受支持")),
                None => {}
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn launch_schema() -> ObjectSchema {
        ObjectSchema::from_json(
            r#"{
                "type": "object",
                "properties": {
                    "package_name": { "type": "string", "pattern": "^\\S+$" }
                },
                "required": ["package_name"],
                "additionalProperties": false
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn accepts_valid_arguments() {
        let schema = launch_schema();
        let errors = schema.validate(&json!({ "package_name": "com.android.settings" }));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn rejects_missing_required() {
        let errors = launch_schema().validate(&json!({}));
        assert_eq!(errors, vec!["缺少必填参数 package_name"]);
    }

    #[test]
    fn rejects_pattern_violation() {
        let errors = launch_schema().validate(&json!({ "package_name": "a b" }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("pattern"));
    }

    #[test]
    fn rejects_additional_property() {
        let errors =
            launch_schema().validate(&json!({ "package_name": "com.x", "evil": "rm -rf /" }));
        assert_eq!(errors, vec!["不允许的额外参数 evil"]);
    }

    #[test]
    fn rejects_wrong_type() {
        let errors = launch_schema().validate(&json!({ "package_name": 42 }));
        assert_eq!(errors, vec!["参数 package_name 必须是 string"]);
    }

    #[test]
    fn empty_object_schema_accepts_empty_args() {
        let schema = ObjectSchema::from_json(
            r#"{ "type": "object", "properties": {}, "additionalProperties": false }"#,
        )
        .unwrap();
        assert!(schema.validate(&json!({})).is_empty());
        assert_eq!(schema.validate(&json!({ "x": 1 })).len(), 1);
    }
}
