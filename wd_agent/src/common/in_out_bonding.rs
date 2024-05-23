use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CommonInput<T> {
    vars: HashMap<String, Value>,
    custom: Option<T>,
}
