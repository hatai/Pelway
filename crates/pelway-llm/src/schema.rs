//! JSONスキーマの検証（Pelway 側、提供元を問わず実施する。ADR 0008）。
//!
//! Z.ai の公式仕様が `response_format` に `json_object` しか挙げないため、
//! どの提供元でもクライアント側の検証が必要
//! （docs/research/rust-llm-client-comparison.md）。
//!
//! 対応は JSON Schema の部分集合: `type`（`string`/`integer`/`number`/`boolean`/
//! `object`/`array`/`null`、配列による複数指定も可）、`enum`、`properties`、`required`、
//! `additionalProperties: false`、`items`。`title`・`description`・`$schema` など
//! 検証に影響しないキーワードは無視する。測定用データのスキーマ
//! （ネストした object・array・文字列・数値・enum）をカバーする範囲で、
//! `oneOf`/`anyOf`/`$ref`/`pattern` 等は検証しない。未知のキーワードは無視するが、
//! `type` の値として未知の名前（誤記を含む）は検証できないためスキーマエラーとして
//! 拒否する（fail-closed）。

use serde_json::Value;

/// `instance` が `schema`（部分集合）に従うか。従わなければ最初の違反を返す。
///
/// 未対応の `type` 名（誤記を含む）は、何でも通してしまうことのないよう
/// スキーマエラーとして拒否する（fail-closed）。
pub fn validate(instance: &Value, schema: &Value) -> Result<(), String> {
    let Some(object) = schema.as_object() else {
        return Err("schema が JSON オブジェクトではありません".to_string());
    };
    if let Some(types) = allowed_types(object.get("type"))
        && !types.is_empty()
    {
        for name in &types {
            if !KNOWN_TYPES.contains(&name.as_str()) {
                return Err(format!(
                    "スキーマの type「{name}」は未対応です（検証できないため拒否します）"
                ));
            }
        }
        if !types.iter().any(|t| type_matches(instance, t)) {
            return Err(format!(
                "type が違います: 期待 {}、実際 {}",
                types.join("|"),
                kind(instance)
            ));
        }
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array)
        && !values.contains(instance)
    {
        return Err(format!("enum に含まれない値です: {instance}"));
    }
    if let Value::Object(fields) = instance {
        if let Some(properties) = object.get("properties").and_then(Value::as_object) {
            for (name, subschema) in properties {
                if let Some(value) = fields.get(name) {
                    validate(value, subschema).map_err(|m| format!(".{name}: {m}"))?;
                }
            }
        }
        if let Some(required) = object.get("required").and_then(Value::as_array) {
            for name in required {
                if let Some(name) = name.as_str()
                    && !fields.contains_key(name)
                {
                    return Err(format!("必須フィールド {name} がありません"));
                }
            }
        }
        if object.get("additionalProperties") == Some(&Value::Bool(false))
            && let Some(properties) = object.get("properties").and_then(Value::as_object)
        {
            for name in fields.keys() {
                if !properties.contains_key(name) {
                    return Err(format!("追加のフィールド {name} は許可されていません"));
                }
            }
        }
    }
    if let Value::Array(items) = instance
        && let Some(subschema) = object.get("items")
    {
        for (i, item) in items.iter().enumerate() {
            validate(item, subschema).map_err(|m| format!("[{i}]: {m}"))?;
        }
    }
    Ok(())
}

/// この検証が扱う `type` の名前。
const KNOWN_TYPES: [&str; 7] = [
    "object", "array", "string", "integer", "number", "boolean", "null",
];

fn allowed_types(value: Option<&Value>) -> Option<Vec<String>> {
    match value? {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(a) => Some(
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ),
        _ => None,
    }
}

fn type_matches(value: &Value, name: &str) -> bool {
    match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        // JSON Schema では 5.0 のような数も integer とみなす。
        "integer" => value.as_f64().is_some_and(|n| n.fract() == 0.0),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        // 未知の名前に一致させることはない（validate が事前に拒否するが、
        // 直接呼ばれても誤って通さないための防御）。
        _ => false,
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
