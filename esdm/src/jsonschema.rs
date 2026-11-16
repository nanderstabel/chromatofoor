use crate::DataSchema;
use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject};

pub trait ToJsonSchema {
    fn to_json_schema(&self) -> Schema;
}

impl ToJsonSchema for DataSchema {
    fn to_json_schema(&self) -> Schema {
        let mut schema_obj = SchemaObject::default();

        let mut object_validation = ObjectValidation::default();

        object_validation
            .required
            .extend(self.required.iter().cloned());

        for (prop_name, prop_schema) in &self.properties {
            let instance_type = match prop_schema.prop_type.as_deref() {
                Some("string") => InstanceType::String,
                Some("integer") => InstanceType::Integer,
                Some("number") => InstanceType::Number,
                Some("boolean") => InstanceType::Boolean,
                Some("array") => InstanceType::Array,
                Some("object") => InstanceType::Object,
                _ => InstanceType::String,
            };

            let prop_schema_obj = SchemaObject {
                instance_type: Some(instance_type.into()),
                ..Default::default()
            };

            object_validation
                .properties
                .insert(prop_name.clone(), Schema::Object(prop_schema_obj));
        }

        schema_obj.instance_type = Some(InstanceType::Object.into());
        schema_obj.object = Some(Box::new(object_validation));

        Schema::Object(schema_obj)
    }
}
