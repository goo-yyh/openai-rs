use serde_json::{Map, Value};

pub(super) fn ensure_vec_len(values: &mut Vec<Value>, len: usize) {
    while values.len() < len {
        values.push(Value::Null);
    }
}

pub(super) fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("value must be object")
}

pub(super) fn ensure_array_field<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    let object = ensure_object(value);
    let field = object
        .entry(key.to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !field.is_array() {
        *field = Value::Array(Vec::new());
    }
    field.as_array_mut().expect("field must be array")
}

pub(super) fn ensure_object_field<'a>(
    value: &'a mut Value,
    key: &str,
) -> &'a mut Map<String, Value> {
    let object = ensure_object(value);
    let field = object
        .entry(key.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    ensure_object(field)
}

pub(super) fn merge_object(target: &mut Map<String, Value>, delta: &Value) {
    let Some(delta_object) = delta.as_object() else {
        return;
    };
    for (key, value) in delta_object {
        if matches!(value, Value::Null) {
            continue;
        }
        target.insert(key.clone(), value.clone());
    }
}
