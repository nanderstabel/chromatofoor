use convert_case::{Case, Casing};
use esdm::{DataSchema, PropertySchema};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;

pub fn collect_prop_ref_idents(prop: &PropertySchema, out: &mut BTreeSet<String>) {
    if let Some(ref ref_path) = prop.ref_path {
        let type_name = ref_path.split('/').next_back().unwrap_or(ref_path);
        out.insert(type_name.to_case(Case::Pascal));
    }
    if let Some(ref items) = prop.items {
        collect_prop_ref_idents(items, out);
    }
    if let Some(ref props) = prop.properties {
        for child in props.values() {
            collect_prop_ref_idents(child, out);
        }
    }
}

pub fn collect_schema_ref_idents(schema: &DataSchema) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for prop in schema.properties.values() {
        collect_prop_ref_idents(prop, &mut out);
    }
    out
}

pub fn emit_domain_ref_imports(refs: &BTreeSet<String>, exclude: &[&str]) -> TokenStream {
    let idents: Vec<syn::Ident> = refs
        .iter()
        .filter(|name| !exclude.contains(&name.as_str()))
        .map(|name| format_ident!("{name}"))
        .collect();
    if idents.is_empty() {
        quote! {}
    } else if idents.len() == 1 {
        let single = &idents[0];
        quote! { use crate::generated::domain::#single; }
    } else {
        quote! { use crate::generated::domain::{#(#idents),*}; }
    }
}

pub fn map_type(prop: &PropertySchema) -> TokenStream {
    if let Some(ref ref_path) = prop.ref_path {
        let type_name = ref_path.split('/').next_back().unwrap_or(ref_path);
        let struct_ident = format_ident!("{}", type_name.to_case(Case::Pascal));
        return quote! { #struct_ident };
    }

    let prop_type = prop.prop_type.as_deref().unwrap_or("object");
    if prop_type == "string" {
        if let Some(ref format) = prop.format {
            if format == "uuid" {
                return quote! { uuid::Uuid };
            } else if format == "identifier" {
                return quote! { chromatofoor::Identifier };
            } else if format == "date" {
                return quote! { chrono::NaiveDate };
            } else if format == "date-time" {
                return quote! { chrono::DateTime<chrono::Utc> };
            }
        }
        return quote! { String };
    } else if prop_type == "integer" {
        return quote! { i64 };
    } else if prop_type == "number" {
        return quote! { f64 };
    } else if prop_type == "boolean" {
        return quote! { bool };
    }
    quote! { String }
}

pub fn generate_struct_fields(
    schema: &DataSchema,
    parent_name: &str,
    extra_structs: &mut Vec<TokenStream>,
) -> Vec<TokenStream> {
    let mut fields = Vec::new();
    let mut props: Vec<_> = schema.properties.iter().collect();
    props.sort_by_key(|(k, _)| *k);

    for (field_name, prop) in props {
        let is_required = schema.required.contains(field_name);

        let (base_type, mut generated_structs) =
            generate_type_for_prop(field_name, prop, parent_name);
        extra_structs.append(&mut generated_structs);

        let final_type = if is_required {
            base_type
        } else {
            quote! { Option<#base_type> }
        };

        let field_ident = format_ident!("{}", field_name.to_case(Case::Snake));

        let rename_attr = if field_name.to_case(Case::Snake) == *field_name {
            quote! {}
        } else {
            quote! { #[serde(rename = #field_name)] }
        };

        let mut validate_attrs = Vec::new();
        if let Some(min) = prop.min_length {
            let min = min as u64;
            if let Some(max) = prop.max_length {
                let max = max as u64;
                validate_attrs.push(quote! { length(min = #min, max = #max) });
            } else {
                validate_attrs.push(quote! { length(min = #min) });
            }
        } else if let Some(max) = prop.max_length {
            let max = max as u64;
            validate_attrs.push(quote! { length(max = #max) });
        }

        if let Some(min_f64) = prop.minimum {
            let min_tok = if prop.prop_type.as_deref() == Some("integer") {
                let m = min_f64 as i64;
                quote! { #m }
            } else {
                quote! { #min_f64 }
            };

            if let Some(max_f64) = prop.maximum {
                let max_tok = if prop.prop_type.as_deref() == Some("integer") {
                    let m = max_f64 as i64;
                    quote! { #m }
                } else {
                    quote! { #max_f64 }
                };
                let msg = format!("Field '{field_name}' must be between {min_f64} and {max_f64}");
                validate_attrs
                    .push(quote! { range(min = #min_tok, max = #max_tok, message = #msg) });
            } else {
                let msg = format!("Field '{field_name}' must be at least {min_f64}");
                validate_attrs.push(quote! { range(min = #min_tok, message = #msg) });
            }
        } else if let Some(max_f64) = prop.maximum {
            let max_tok = if prop.prop_type.as_deref() == Some("integer") {
                let m = max_f64 as i64;
                quote! { #m }
            } else {
                quote! { #max_f64 }
            };
            let msg = format!("Field '{field_name}' must be at most {max_f64}");
            validate_attrs.push(quote! { range(max = #max_tok, message = #msg) });
        }

        if prop.ref_path.is_some() || prop.prop_type.as_deref() == Some("object") {
            validate_attrs.push(quote! { nested });
        } else if prop.prop_type.as_deref() == Some("array")
            && let Some(items) = &prop.items
            && (items.ref_path.is_some() || items.prop_type.as_deref() == Some("object"))
        {
            validate_attrs.push(quote! { nested });
        }

        let validate_attr = if validate_attrs.is_empty() {
            quote! {}
        } else {
            quote! { #[validate(#(#validate_attrs),*)] }
        };

        fields.push(quote! {
            #rename_attr
            #validate_attr
            pub #field_ident: #final_type
        });
    }
    fields
}

pub fn generate_type_for_prop(
    field_name: &str,
    prop: &PropertySchema,
    parent_name: &str,
) -> (TokenStream, Vec<TokenStream>) {
    if let Some(ref ref_path) = prop.ref_path {
        let type_name = ref_path.split('/').next_back().unwrap_or(ref_path);
        let struct_ident = format_ident!("{}", type_name.to_case(Case::Pascal));
        return (quote! { #struct_ident }, vec![]);
    }

    let prop_type = prop.prop_type.as_deref().unwrap_or("object");
    let mut extra_structs = Vec::new();
    let type_ts = if let Some(ref enum_vals) = prop.enum_values {
        let enum_name = format!("{}{}", parent_name, field_name.to_case(Case::Pascal));
        let enum_ident = format_ident!("{}", enum_name);

        let mut sorted_vals = enum_vals.clone();
        sorted_vals.sort();

        let mut first = true;
        let variant_tokens = sorted_vals.iter().map(|v| {
            let sanitized: String = v
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect();
            let pascal = sanitized.to_case(Case::Pascal);
            let final_ident = if pascal.chars().next().unwrap_or(' ').is_numeric() {
                format!("_{pascal}")
            } else if pascal.is_empty() {
                "Empty".to_string()
            } else {
                pascal
            };
            let ident = format_ident!("{}", final_ident);
            let default_attr = if first {
                first = false;
                quote! { #[default] }
            } else {
                quote! {}
            };
            quote! {
                #default_attr
                #[serde(rename = #v)]
                #ident
            }
        });

        extra_structs.push(quote! {
            #[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
            pub enum #enum_ident {
                #(#variant_tokens),*
            }
            impl chromatofoor::architecturekit::Schema for #enum_ident {
                fn schema() -> chromatofoor::serde_json::Value {
                    chromatofoor::serde_json::json!({
                        "type": "string",
                        "enum": [#(#sorted_vals),*],
                    })
                }
            }
        });
        quote! { #enum_ident }
    } else if prop_type == "object" {
        let struct_name = format!("{}{}", parent_name, field_name.to_case(Case::Pascal));
        let struct_ident = format_ident!("{}", struct_name);

        let mut inner_fields = Vec::new();
        if let Some(props) = &prop.properties {
            let required = prop.required.clone().unwrap_or_default();
            let inner_schema = DataSchema {
                properties: props.clone(),
                required,
                example: None,
            };
            inner_fields = generate_struct_fields(&inner_schema, &struct_name, &mut extra_structs);
        }

        extra_structs.push(quote! {
            #[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, chromatofoor::validator::Validate, chromatofoor::architecturekit::Schema)]
            #[validate(crate = "chromatofoor::validator")]
            #[architecturekit(crate = "chromatofoor::architecturekit")]
            pub struct #struct_ident {
                #(#inner_fields),*
            }
        });

        quote! { #struct_ident }
    } else if prop_type == "array" {
        if let Some(items) = &prop.items {
            let (item_type, mut item_structs) = generate_type_for_prop(
                &format!("{}Item", field_name.to_case(Case::Pascal)),
                items,
                parent_name,
            );
            extra_structs.append(&mut item_structs);
            quote! { Vec<#item_type> }
        } else {
            quote! { Vec<String> }
        }
    } else {
        map_type(prop)
    };

    (type_ts, extra_structs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::PropertySchema;
    use quote::ToTokens;

    #[test]
    fn test_map_type_primitive() {
        let prop = PropertySchema {
            prop_type: Some("string".to_string()),
            ..Default::default()
        };
        let ty = map_type(&prop);
        assert_eq!(ty.into_token_stream().to_string(), "String");
    }

    #[test]
    fn test_map_type_format_date_time() {
        let prop = PropertySchema {
            prop_type: Some("string".to_string()),
            format: Some("date-time".to_string()),
            ..Default::default()
        };
        let ty = map_type(&prop);
        assert_eq!(
            ty.into_token_stream().to_string(),
            "chrono :: DateTime < chrono :: Utc >"
        );
    }

    #[test]
    fn test_map_type_format_identifier() {
        let prop = PropertySchema {
            prop_type: Some("string".to_string()),
            format: Some("identifier".to_string()),
            ..Default::default()
        };
        let ty = map_type(&prop);
        assert_eq!(
            ty.into_token_stream().to_string(),
            "chromatofoor :: Identifier"
        );
    }

    #[test]
    fn test_generate_type_for_prop_array() {
        let items_prop = Box::new(PropertySchema {
            prop_type: Some("integer".to_string()),
            ..Default::default()
        });

        let prop = PropertySchema {
            prop_type: Some("array".to_string()),
            items: Some(items_prop),
            ..Default::default()
        };

        let (ty, structs) = generate_type_for_prop("MyArray", &prop, "TestParent");
        assert_eq!(ty.into_token_stream().to_string(), "Vec < i64 >");
        assert!(structs.is_empty());
    }

    #[test]
    fn test_generate_type_for_prop_enum_emits_manual_schema_impl() {
        let prop = PropertySchema {
            prop_type: Some("string".to_string()),
            enum_values: Some(vec!["pending".to_string(), "active".to_string()]),
            ..Default::default()
        };

        let (ty, structs) = generate_type_for_prop("status", &prop, "Order");
        assert_eq!(ty.into_token_stream().to_string(), "OrderStatus");
        assert_eq!(structs.len(), 1);
        let generated = structs[0].to_string();
        assert!(
            generated.contains("impl chromatofoor :: architecturekit :: Schema for OrderStatus")
        );
        assert!(!generated.contains("derive (Default , Debug , Clone , serde :: Serialize , serde :: Deserialize , PartialEq , Eq , chromatofoor :: architecturekit :: Schema)"));
    }
}
