use async_graphql::dynamic::{
    Field, FieldFuture, FieldValue, InputObject, InputValue, Object, TypeRef,
};
use convert_case::{Case, Casing};
use serde_json::Value;

/// Classification of how a JSON value should be wrapped into an [`async_graphql::dynamic::FieldValue`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputShape {
    /// Scalar (`String`, `Int`, `Float`, `Boolean`, `JSON`) or list of scalars.
    Scalar,
    /// Typed GraphQL `Object` backed by a `serde_json::Value::Object`.
    Object,
    /// List of typed GraphQL `Object`s (`[Object!]`) backed by a `serde_json::Value::Array`.
    ObjectList,
}

/// Converts a `serde_json::Value` into an [`async_graphql::dynamic::FieldValue`] matching `shape`.
pub fn json_to_field_value(
    val: Value,
    shape: OutputShape,
) -> Result<Option<FieldValue<'static>>, async_graphql::Error> {
    if val.is_null() {
        return Ok(None);
    }
    match shape {
        OutputShape::Scalar => Ok(Some(FieldValue::value(async_graphql::Value::from_json(
            val,
        )?))),
        OutputShape::Object => Ok(Some(FieldValue::owned_any(val))),
        OutputShape::ObjectList => {
            if let Value::Array(items) = val {
                Ok(Some(FieldValue::list(
                    items
                        .into_iter()
                        .filter(|item| !item.is_null())
                        .map(FieldValue::owned_any),
                )))
            } else {
                Ok(None)
            }
        }
    }
}

/// Returns `true` if `prop` explicitly permits `null` (via `type: [..., "null"]` or `anyOf`/`oneOf`).
#[must_use]
pub fn is_nullable(prop: &Value) -> bool {
    if let Some(arr) = prop.get("type").and_then(Value::as_array)
        && arr.iter().any(|v| v.as_str() == Some("null"))
    {
        return true;
    }
    if let Some(branches) = prop
        .get("anyOf")
        .or_else(|| prop.get("oneOf"))
        .and_then(Value::as_array)
        && branches
            .iter()
            .any(|b| b.get("type").and_then(Value::as_str) == Some("null"))
    {
        return true;
    }
    false
}

fn resolve_schema_with_depth<'a>(
    prop: &'a Value,
    root_schema: &'a Value,
    depth: usize,
) -> &'a Value {
    if depth > 8 {
        return prop;
    }
    if let Some(ref_str) = prop.get("$ref").and_then(Value::as_str) {
        let def_name = ref_str
            .strip_prefix("#/$defs/")
            .or_else(|| ref_str.strip_prefix("#/definitions/"));
        if let Some(name) = def_name
            && let Some(resolved) = root_schema
                .get("$defs")
                .or_else(|| root_schema.get("definitions"))
                .and_then(|defs| defs.get(name))
        {
            return resolve_schema_with_depth(resolved, root_schema, depth + 1);
        }
    }
    if let Some(all_of) = prop.get("allOf").and_then(Value::as_array)
        && all_of.len() == 1
    {
        return resolve_schema_with_depth(&all_of[0], root_schema, depth + 1);
    }
    if let Some(branches) = prop
        .get("anyOf")
        .or_else(|| prop.get("oneOf"))
        .and_then(Value::as_array)
        && let Some(non_null) = branches
            .iter()
            .find(|b| b.get("type").and_then(Value::as_str) != Some("null"))
    {
        return resolve_schema_with_depth(non_null, root_schema, depth + 1);
    }
    prop
}

/// Resolves a `$ref`, single-element `allOf`, or nullable `anyOf`/`oneOf` schema node against `root_schema`.
#[must_use]
pub fn resolve_schema<'a>(prop: &'a Value, root_schema: &'a Value) -> &'a Value {
    resolve_schema_with_depth(prop, root_schema, 0)
}

/// Extracts the primary JSON Schema type name from `prop`, unwrapping `["<type>", "null"]` arrays.
#[must_use]
pub fn effective_type(prop: &Value) -> &str {
    if let Some(ty) = prop.get("type") {
        if let Some(s) = ty.as_str() {
            return s;
        }
        if let Some(arr) = ty.as_array()
            && let Some(non_null) = arr.iter().filter_map(Value::as_str).find(|s| *s != "null")
        {
            return non_null;
        }
    }
    if prop.get("properties").is_some() {
        return "object";
    }
    "object"
}

pub fn build_type_ref(prop: &Value, is_required: bool) -> TypeRef {
    let mut current = match effective_type(prop) {
        "string" => TypeRef::Named(TypeRef::STRING.into()),
        "integer" => TypeRef::Named(TypeRef::INT.into()),
        "number" => TypeRef::Named(TypeRef::FLOAT.into()),
        "boolean" => TypeRef::Named(TypeRef::BOOLEAN.into()),
        "array" => {
            if let Some(items) = prop.get("items") {
                let inner = build_type_ref(items, !is_nullable(items));
                TypeRef::List(Box::new(inner))
            } else {
                TypeRef::List(Box::new(TypeRef::Named("JSON".into())))
            }
        }
        _ => TypeRef::Named("JSON".into()),
    };

    if is_required {
        current = TypeRef::NonNull(Box::new(current));
    }

    current
}

fn build_input_type_ref_recursive(
    parent_name: &str,
    field_name: &str,
    raw_prop: &Value,
    root_schema: &Value,
    is_required: bool,
    depth: usize,
    out: &mut Vec<InputObject>,
) -> TypeRef {
    if depth > 8 {
        let base = TypeRef::Named("JSON".into());
        return if is_required {
            TypeRef::NonNull(Box::new(base))
        } else {
            base
        };
    }

    let prop = resolve_schema_with_depth(raw_prop, root_schema, depth);
    let mut current = match effective_type(prop) {
        "string" => TypeRef::Named(TypeRef::STRING.into()),
        "integer" => TypeRef::Named(TypeRef::INT.into()),
        "number" => TypeRef::Named(TypeRef::FLOAT.into()),
        "boolean" => TypeRef::Named(TypeRef::BOOLEAN.into()),
        "array" => {
            if let Some(raw_items) = prop.get("items") {
                let items = resolve_schema_with_depth(raw_items, root_schema, depth + 1);
                if depth < 8
                    && effective_type(items) == "object"
                    && items
                        .get("properties")
                        .and_then(Value::as_object)
                        .is_some_and(|p| !p.is_empty())
                {
                    let nested_name = format!(
                        "{}{}ItemInput",
                        parent_name.strip_suffix("Input").unwrap_or(parent_name),
                        field_name.to_case(Case::Pascal)
                    );
                    create_input_objects_into(&nested_name, items, root_schema, depth + 1, out);
                    let item_ref = TypeRef::Named(nested_name.into());
                    let item_ref = if is_nullable(raw_items) {
                        item_ref
                    } else {
                        TypeRef::NonNull(Box::new(item_ref))
                    };
                    TypeRef::List(Box::new(item_ref))
                } else {
                    let inner = build_type_ref(items, !is_nullable(raw_items));
                    TypeRef::List(Box::new(inner))
                }
            } else {
                TypeRef::List(Box::new(TypeRef::Named("JSON".into())))
            }
        }
        "object"
            if depth < 8
                && prop
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|p| !p.is_empty()) =>
        {
            let nested_name = format!(
                "{}{}Input",
                parent_name.strip_suffix("Input").unwrap_or(parent_name),
                field_name.to_case(Case::Pascal)
            );
            create_input_objects_into(&nested_name, prop, root_schema, depth + 1, out);
            TypeRef::Named(nested_name.into())
        }
        _ => TypeRef::Named("JSON".into()),
    };

    if is_required {
        current = TypeRef::NonNull(Box::new(current));
    }

    current
}

fn create_input_objects_into(
    name: &str,
    node_schema: &Value,
    root_schema: &Value,
    depth: usize,
    out: &mut Vec<InputObject>,
) {
    if depth > 8 {
        return;
    }
    let mut obj = InputObject::new(name);
    let required: Vec<&str> = node_schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if let Some(props) = node_schema.get("properties").and_then(Value::as_object) {
        for (field_name, prop) in props {
            let is_req = required.contains(&field_name.as_str()) && !is_nullable(prop);
            let type_ref = build_input_type_ref_recursive(
                name,
                field_name,
                prop,
                root_schema,
                is_req,
                depth,
                out,
            );
            let mut input_val = InputValue::new(field_name.to_case(Case::Camel), type_ref);
            if let Some(desc) = prop.get("description").and_then(Value::as_str) {
                input_val = input_val.description(desc);
            }
            obj = obj.field(input_val);
        }
    }
    out.push(obj);
}

/// Creates the root [`InputObject`] and any nested [`InputObject`]s required for `schema`.
#[must_use]
pub fn create_input_objects(name: &str, schema: &Value) -> Vec<InputObject> {
    let mut out = Vec::new();
    let resolved = resolve_schema(schema, schema);
    create_input_objects_into(name, resolved, schema, 0, &mut out);
    out
}

pub fn create_input_object(name: &str, schema: &Value) -> InputObject {
    create_input_objects(name, schema)
        .pop()
        .unwrap_or_else(|| InputObject::new(name))
}

fn build_output_type_ref_recursive(
    parent_name: &str,
    field_name: &str,
    raw_prop: &Value,
    root_schema: &Value,
    is_required: bool,
    depth: usize,
    out: &mut Vec<Object>,
) -> (TypeRef, OutputShape) {
    if depth > 8 {
        let base = TypeRef::Named("JSON".into());
        let ty = if is_required {
            TypeRef::NonNull(Box::new(base))
        } else {
            base
        };
        return (ty, OutputShape::Scalar);
    }

    let prop = resolve_schema_with_depth(raw_prop, root_schema, depth);
    let base_prefix = parent_name.strip_suffix("Output").unwrap_or(parent_name);
    let (mut current, shape) = match effective_type(prop) {
        "string" => (TypeRef::Named(TypeRef::STRING.into()), OutputShape::Scalar),
        "integer" => (TypeRef::Named(TypeRef::INT.into()), OutputShape::Scalar),
        "number" => (TypeRef::Named(TypeRef::FLOAT.into()), OutputShape::Scalar),
        "boolean" => (TypeRef::Named(TypeRef::BOOLEAN.into()), OutputShape::Scalar),
        "array" => {
            if let Some(raw_items) = prop.get("items") {
                let items = resolve_schema_with_depth(raw_items, root_schema, depth + 1);
                if depth < 8
                    && effective_type(items) == "object"
                    && items
                        .get("properties")
                        .and_then(Value::as_object)
                        .is_some_and(|p| !p.is_empty())
                {
                    let nested_name = if field_name.is_empty() {
                        format!("{base_prefix}ItemOutput")
                    } else {
                        format!(
                            "{base_prefix}{}ItemOutput",
                            field_name.to_case(Case::Pascal)
                        )
                    };
                    create_output_objects_into(&nested_name, items, root_schema, depth + 1, out);
                    let item_ref = TypeRef::NonNull(Box::new(TypeRef::Named(nested_name.into())));
                    (TypeRef::List(Box::new(item_ref)), OutputShape::ObjectList)
                } else {
                    let inner = build_type_ref(items, !is_nullable(raw_items));
                    (TypeRef::List(Box::new(inner)), OutputShape::Scalar)
                }
            } else {
                (
                    TypeRef::List(Box::new(TypeRef::Named("JSON".into()))),
                    OutputShape::Scalar,
                )
            }
        }
        "object"
            if depth < 8
                && prop
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|p| !p.is_empty()) =>
        {
            let nested_name = if field_name.is_empty() {
                parent_name.to_owned()
            } else {
                format!("{base_prefix}{}Output", field_name.to_case(Case::Pascal))
            };
            create_output_objects_into(&nested_name, prop, root_schema, depth + 1, out);
            (TypeRef::Named(nested_name.into()), OutputShape::Object)
        }
        _ => (TypeRef::Named("JSON".into()), OutputShape::Scalar),
    };

    if is_required {
        current = TypeRef::NonNull(Box::new(current));
    }

    (current, shape)
}

fn create_output_objects_into(
    name: &str,
    node_schema: &Value,
    root_schema: &Value,
    depth: usize,
    out: &mut Vec<Object>,
) {
    if depth > 8 {
        return;
    }
    let mut obj = Object::new(name);
    let required: Vec<&str> = node_schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    if let Some(props) = node_schema.get("properties").and_then(Value::as_object) {
        for (field_name, prop) in props {
            let is_req = required.contains(&field_name.as_str()) && !is_nullable(prop);
            let (type_ref, shape) = build_output_type_ref_recursive(
                name,
                field_name,
                prop,
                root_schema,
                is_req,
                depth,
                out,
            );

            let field_name_snake = field_name.clone();
            let field_name_camel = field_name.to_case(Case::Camel);
            let mut field = Field::new(field_name_camel.clone(), type_ref, move |ctx| {
                let key_snake = field_name_snake.clone();
                let key_camel = field_name_camel.clone();
                FieldFuture::new(async move {
                    let parent = ctx.parent_value.try_downcast_ref::<Value>()?;
                    let val = parent
                        .get(&key_snake)
                        .or_else(|| parent.get(&key_camel))
                        .cloned()
                        .unwrap_or(Value::Null);
                    json_to_field_value(val, shape)
                })
            });
            if let Some(desc) = prop.get("description").and_then(Value::as_str) {
                field = field.description(desc);
            }
            obj = obj.field(field);
        }
    }
    out.push(obj);
}

/// Builds the GraphQL output [`TypeRef`], [`OutputShape`], and any generated [`Object`] types
/// for a query's `output_schema`.
#[must_use]
pub fn build_query_output_type(
    base_name: &str,
    output_schema: &Value,
) -> (TypeRef, OutputShape, Vec<Object>) {
    let mut out = Vec::new();
    let is_req = !is_nullable(output_schema);
    let (type_ref, shape) = build_output_type_ref_recursive(
        base_name,
        "",
        output_schema,
        output_schema,
        is_req,
        0,
        &mut out,
    );
    (type_ref, shape, out)
}

pub fn create_object(name: &str, schema: &Value) -> Object {
    let mut out = Vec::new();
    let resolved = resolve_schema(schema, schema);
    create_output_objects_into(name, resolved, schema, 0, &mut out);
    out.pop().unwrap_or_else(|| Object::new(name))
}
