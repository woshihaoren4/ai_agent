use prost_types::value::Kind;
use prost_types::{ListValue, Struct, Value as ProstValue};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

pub fn prost_struct_to_serde_value(p_struct: Struct) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in p_struct.fields {
        map.insert(key, prost_value_to_serde_value(value));
    }
    Value::Object(map)
}

pub fn prost_value_to_serde_value(p_value: prost_types::Value) -> Value {
    match p_value.kind {
        Some(Kind::NullValue(_)) => Value::Null,
        Some(Kind::NumberValue(n)) => Value::Number(Number::from_f64(n).unwrap_or(Number::from(0))),
        Some(Kind::StringValue(s)) => Value::String(s),
        Some(Kind::BoolValue(b)) => Value::Bool(b),
        Some(Kind::StructValue(s)) => prost_struct_to_serde_value(s),
        Some(Kind::ListValue(list)) => Value::Array(
            list.values
                .into_iter()
                .map(prost_value_to_serde_value)
                .collect(),
        ),
        None => Value::Null,
    }
}

pub fn serde_value_to_prost_value(value: &Value) -> ProstValue {
    match value {
        serde_json::Value::Null => ProstValue {
            kind: Some(prost_types::value::Kind::NullValue(0)),
        },
        serde_json::Value::Bool(b) => ProstValue {
            kind: Some(prost_types::value::Kind::BoolValue(*b)),
        },
        serde_json::Value::Number(num) => {
            if let Some(n) = num.as_i64() {
                ProstValue {
                    kind: Some(prost_types::value::Kind::NumberValue(n as f64)),
                }
            } else if let Some(n) = num.as_f64() {
                ProstValue {
                    kind: Some(prost_types::value::Kind::NumberValue(n)),
                }
            } else {
                panic!("Invalid number value: {}", num);
            }
        }
        serde_json::Value::String(s) => ProstValue {
            kind: Some(prost_types::value::Kind::StringValue(s.clone())),
        },
        serde_json::Value::Array(arr) => {
            let values = arr.iter().map(serde_value_to_prost_value).collect();
            ProstValue {
                kind: Some(prost_types::value::Kind::ListValue(ListValue { values })),
            }
        }
        serde_json::Value::Object(obj) => {
            let mut fields = BTreeMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), serde_value_to_prost_value(v));
            }
            ProstValue {
                kind: Some(prost_types::value::Kind::StructValue(Struct { fields })),
            }
        }
    }
}

pub fn serde_value_to_prost_struct(value: &Value) -> Option<Struct> {
    let pv = serde_value_to_prost_value(value);
    if let Some(Kind::StructValue(s)) = pv.kind {
        Some(s)
    } else {
        None
    }
}
