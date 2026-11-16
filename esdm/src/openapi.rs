#![allow(deprecated)]
use crate::DataSchema;
use utoipa::openapi::schema::{ObjectBuilder, Schema, SchemaType, Type};

pub trait ToOpenApi {
    fn to_openapi_schema(&self) -> Schema;
}

impl ToOpenApi for DataSchema {
    fn to_openapi_schema(&self) -> Schema {
        let mut builder = ObjectBuilder::new().schema_type(SchemaType::Type(Type::Object));

        for req in &self.required {
            builder = builder.required(req.clone());
        }

        if let Some(example) = &self.example
            && let Ok(json_val) = serde_json::to_value(example)
        {
            builder = builder.example(Some(json_val));
        }

        for (prop_name, prop_schema) in &self.properties {
            let prop_type = match prop_schema.prop_type.as_deref() {
                Some("integer") => Type::Integer,
                Some("number") => Type::Number,
                Some("boolean") => Type::Boolean,
                Some("array") => Type::Array,
                Some("object") => Type::Object,
                _ => Type::String,
            };

            let mut prop_builder = ObjectBuilder::new().schema_type(SchemaType::Type(prop_type));

            if let Some(example) = &prop_schema.example
                && let Ok(json_val) = serde_json::to_value(example)
            {
                prop_builder = prop_builder.example(Some(json_val));
            }

            builder = builder.property(prop_name, prop_builder.build());
        }

        builder.build().into()
    }
}
