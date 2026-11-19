use std::cell::RefCell;

use serde_json::{Map, Value, json};

thread_local! {
    static DERIVING: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// Evaluates `describe` while guarding against recursive type schema expansion.
#[doc(hidden)]
#[must_use]
pub fn derive_schema(type_name: &'static str, describe: impl FnOnce() -> Value) -> Value {
    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            DERIVING.with_borrow_mut(|stack| {
                let _ = stack.pop();
            });
        }
    }

    let is_recursive = DERIVING.with_borrow_mut(|stack| {
        if stack.contains(&type_name) {
            return true;
        }
        stack.push(type_name);
        false
    });
    assert!(
        !is_recursive,
        "chromatofoor: {type_name} is recursive, so its schema can not be derived; implement Schema for it by hand",
    );

    let _guard = Guard;
    describe()
}

/// Builds a JSON Schema `object` definition from `(field_name, schema, is_optional)` entries.
#[doc(hidden)]
#[must_use]
pub fn object_schema(type_name: &'static str, fields: Vec<(&'static str, Value, bool)>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for (name, schema, is_optional) in fields {
        assert!(
            properties.insert(name.to_owned(), schema).is_none(),
            "chromatofoor: {type_name} writes the field {name} more than once, so its schema can not be derived",
        );
        if !is_optional {
            required.push(Value::from(name));
        }
    }

    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = Value::Array(required);
    }

    schema
}

/// Builds a JSON Schema string `enum` definition restricted to `variants`.
#[doc(hidden)]
#[must_use]
pub fn string_enum_schema(variants: &[&'static str]) -> Value {
    json!({
        "type": "string",
        "enum": variants,
    })
}
