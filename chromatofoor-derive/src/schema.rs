use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use serde_derive_internals::{
    Ctxt, Derive,
    ast::{Container, Data, Field, Style, Variant},
    attr::TagType,
};
use syn::{
    Data as SynData, DeriveInput, Error, Fields, GenericParam, Result, Type, parse_macro_input,
    parse_quote, spanned::Spanned,
};

pub(crate) fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_schema(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_schema(input: &DeriveInput) -> Result<TokenStream2> {
    let cx = Ctxt::new();
    let parsed = Container::from_ast(&cx, input, Derive::Serialize);
    cx.check()?;
    let Some(container) = parsed else {
        return Err(Error::new(
            input.ident.span(),
            "failed to parse serde attributes for Schema",
        ));
    };

    ensure_container_derivable(&container, input)?;

    let body = match &container.data {
        Data::Struct(style, fields) => schema_of_struct(&container, *style, fields, input)?,
        Data::Enum(variants) => {
            if let Some(param) = input.generics.params.first() {
                return Err(Error::new_spanned(
                    param,
                    "an enum deriving Schema can not be generic",
                ));
            }
            schema_of_unit_enum(variants, input)?
        }
    };

    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(param) = param {
            param
                .bounds
                .push(parse_quote!(chromatofoor::architecturekit::Schema));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let name = &input.ident;

    Ok(quote! {
        impl #impl_generics chromatofoor::architecturekit::Schema for #name #type_generics #where_clause {
            fn schema() -> chromatofoor::serde_json::Value {
                chromatofoor::common::schema::derive_schema(::core::any::type_name::<Self>(), || #body)
            }
        }
    })
}

fn ensure_container_derivable(container: &Container<'_>, input: &DeriveInput) -> Result<()> {
    if container.attrs.type_into().is_some()
        || container.attrs.remote().is_some()
        || !matches!(container.attrs.tag(), TagType::External)
    {
        return Err(Error::new(
            input.ident.span(),
            "the schema of a type with #[serde(tag = ...)], #[serde(content = ...)], #[serde(untagged)], \
             #[serde(into = ...)], or #[serde(remote = ...)] can not be derived; implement architecturekit::Schema by hand",
        ));
    }
    Ok(())
}

fn schema_of_struct(
    container: &Container<'_>,
    style: Style,
    fields: &[Field<'_>],
    input: &DeriveInput,
) -> Result<TokenStream2> {
    if container.attrs.transparent() {
        let written: Vec<_> = fields
            .iter()
            .filter(|f| !f.attrs.skip_serializing())
            .collect();
        return match written.as_slice() {
            [field] => {
                ensure_field_derivable(field)?;
                let field_type = field.ty;
                Ok(quote! { <#field_type as chromatofoor::architecturekit::Schema>::schema() })
            }
            _ => Err(Error::new(
                input.ident.span(),
                "a transparent struct needs exactly one field that is not skipped",
            )),
        };
    }

    match style {
        Style::Struct => {
            let mut entries = Vec::new();
            for field in fields {
                if field.attrs.skip_serializing() {
                    continue;
                }
                ensure_field_derivable(field)?;
                let name = field.attrs.name().serialize_name();
                let field_type = field.ty;
                if field.attrs.skip_serializing_if().is_some() {
                    entries.push(quote! {
                        (#name, <#field_type as chromatofoor::architecturekit::Schema>::schema_without_null(), true)
                    });
                } else {
                    entries.push(quote! {
                        (#name, <#field_type as chromatofoor::architecturekit::Schema>::schema(), false)
                    });
                }
            }
            Ok(quote! {
                chromatofoor::common::schema::object_schema(::core::any::type_name::<Self>(), ::std::vec![#(#entries),*])
            })
        }
        Style::Newtype => {
            let Some(field) = fields.first() else {
                return Err(Error::new(input.ident.span(), "expected a newtype field"));
            };
            ensure_field_derivable(field)?;
            let field_type = field.ty;
            Ok(quote! { <#field_type as chromatofoor::architecturekit::Schema>::schema() })
        }
        Style::Tuple => Err(Error::new(
            input.ident.span(),
            "the schema of a tuple struct with several fields can not be derived, since serde writes it \
             as an array of mixed values; use named fields, or implement architecturekit::Schema by hand",
        )),
        Style::Unit => Err(Error::new(
            input.ident.span(),
            "the schema of a unit struct can not be derived, since serde writes it as null; \
             to describe data without fields, use `struct Name {}`",
        )),
    }
}

fn ensure_field_derivable(field: &Field<'_>) -> Result<()> {
    if field.attrs.flatten() || field.attrs.serialize_with().is_some() {
        return Err(Error::new_spanned(
            field.original,
            "the schema of a field with #[serde(flatten)], #[serde(serialize_with = ...)], or #[serde(with = ...)] \
             can not be derived; implement architecturekit::Schema by hand",
        ));
    }
    Ok(())
}

fn schema_of_unit_enum(variants: &[Variant<'_>], input: &DeriveInput) -> Result<TokenStream2> {
    let mut unit_exprs = Vec::new();
    for variant in variants {
        if !matches!(variant.style, Style::Unit) {
            return Err(Error::new(
                input.ident.span(),
                "the schema of an enum with non-unit variants can not be derived, since serde writes enums in several shapes; \
                 implement architecturekit::Schema by hand",
            ));
        }
        if variant.attrs.untagged() || variant.attrs.serialize_with().is_some() {
            return Err(Error::new_spanned(
                variant.original,
                "the schema of an enum variant with #[serde(untagged)], #[serde(serialize_with = ...)], \
                 or #[serde(with = ...)] can not be derived; implement architecturekit::Schema by hand",
            ));
        }
        if variant.attrs.skip_serializing() {
            continue;
        }
        let v_ident = &variant.ident;
        unit_exprs.push(quote! { Self::#v_ident });
    }

    Ok(quote! {
        chromatofoor::common::schema::unit_enum_schema::<Self>(&[#(#unit_exprs),*])
    })
}

pub(crate) fn derive_events(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_events(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_events(input: &DeriveInput) -> Result<TokenStream2> {
    if let Some(param) = input.generics.params.first() {
        return Err(Error::new(
            param.span(),
            "an enum of events can not be generic, since its variants name the event types of a state",
        ));
    }

    let SynData::Enum(data) = &input.data else {
        return Err(Error::new(
            input.ident.span(),
            "Events can only be derived for an enum with one variant per event type",
        ));
    };
    if data.variants.is_empty() {
        return Err(Error::new(
            input.ident.span(),
            "an enum of events needs at least one variant",
        ));
    }

    let mut variants: Vec<(&syn::Ident, &Type)> = Vec::new();
    for variant in &data.variants {
        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                variants.push((&variant.ident, &fields.unnamed[0].ty));
            }
            _ => {
                return Err(Error::new(
                    variant.span(),
                    "every variant of an enum of events holds exactly one event, as in Acquired(BookAcquired)",
                ));
            }
        }
    }

    let name = &input.ident;
    let event_types = variants
        .iter()
        .map(|(_, event)| quote! { <#event as chromatofoor::architecturekit::Event>::EVENT_TYPE });
    let schemas = variants
        .iter()
        .map(|(_, event)| quote! { chromatofoor::architecturekit::EventSchema::of::<#event>() });
    let event_type_arms = variants.iter().map(|(variant, event)| {
        quote! { Self::#variant(_) => <#event as chromatofoor::architecturekit::Event>::EVENT_TYPE }
    });
    let to_data_arms = variants.iter().map(|(variant, _)| {
        quote! { Self::#variant(event) => chromatofoor::serde_json::to_value(event) }
    });
    let from_data_checks = variants.iter().map(|(variant, event)| {
        quote! {
            if event_type == <#event as chromatofoor::architecturekit::Event>::EVENT_TYPE {
                return ::core::option::Option::Some(
                    chromatofoor::serde_json::from_value::<#event>(data).map(Self::#variant),
                );
            }
        }
    });

    let message = format!("two variants of {name} hold events of the same event type");
    let mut duplicate_checks = Vec::new();
    for (index, (_, left)) in variants.iter().enumerate() {
        for (_, right) in &variants[index + 1..] {
            duplicate_checks.push(quote! {
                ::core::assert!(
                    !chromatofoor::common::const_str_eq(
                        <#left as chromatofoor::architecturekit::Event>::EVENT_TYPE,
                        <#right as chromatofoor::architecturekit::Event>::EVENT_TYPE,
                    ),
                    #message,
                );
            });
        }
    }

    Ok(quote! {
        impl chromatofoor::architecturekit::Events for #name {
            const EVENT_TYPES: &'static [&'static str] = &[#(#event_types),*];

            fn schemas() -> ::std::vec::Vec<chromatofoor::architecturekit::EventSchema> {
                ::std::vec![#(#schemas),*]
            }

            fn event_type(&self) -> &'static str {
                match self {
                    #(#event_type_arms),*
                }
            }

            fn to_data(
                &self,
            ) -> ::core::result::Result<chromatofoor::serde_json::Value, chromatofoor::serde_json::Error> {
                match self {
                    #(#to_data_arms),*
                }
            }

            fn from_data(
                event_type: &str,
                data: chromatofoor::serde_json::Value,
            ) -> ::core::option::Option<::core::result::Result<Self, chromatofoor::serde_json::Error>> {
                #(#from_data_checks)*
                ::core::option::Option::None
            }
        }

        const _: () = {
            #(#duplicate_checks)*
        };
    })
}
