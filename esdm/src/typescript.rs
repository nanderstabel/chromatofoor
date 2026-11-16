use crate::DataSchema;

pub trait ToTypeScript {
    fn to_typescript(&self, name: &str) -> String;
}

impl ToTypeScript for DataSchema {
    fn to_typescript(&self, name: &str) -> String {
        let mut out = format!("export interface {name} {{\n");

        for (prop_name, prop_schema) in &self.properties {
            let is_required = self.required.contains(prop_name);
            let optional_mark = if is_required { "" } else { "?" };

            let ts_type = match prop_schema.prop_type.as_deref() {
                Some("string") => "string",
                Some("integer" | "number") => "number",
                Some("boolean") => "boolean",
                Some("array") => "any[]", // simplified for now
                Some("object") => "Record<string, any>",
                _ => "any",
            };

            out.push_str(&format!("  {prop_name}{optional_mark}: {ts_type};\n"));
        }

        out.push_str("}\n");
        out
    }
}
