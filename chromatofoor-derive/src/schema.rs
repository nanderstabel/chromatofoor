use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, GenericParam, LitStr, Result, Token, Type,
    ext::IdentExt, meta::ParseNestedMeta, parse_macro_input, parse_quote, spanned::Spanned,
};

#[derive(Clone, Copy, Default)]
enum SerdeRenameRule {
    #[default]
    None,
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl SerdeRenameRule {
    fn from_lit(lit: &LitStr) -> Result<Self> {
        match lit.value().as_str() {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            other => Err(Error::new(
                lit.span(),
                format!("unknown serde rename rule `{other}`"),
            )),
        }
    }

    fn apply_to_field(self, field: &str) -> String {
        match self {
            Self::None | Self::LowerCase | Self::SnakeCase => field.to_owned(),
            Self::UpperCase | Self::ScreamingSnakeCase => field.to_ascii_uppercase(),
            Self::PascalCase => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            Self::CamelCase => {
                let pascal = Self::PascalCase.apply_to_field(field);
                let mut chars = pascal.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_lowercase().to_string() + chars.as_str()
                })
            }
            Self::KebabCase => field.replace('_', "-"),
            Self::ScreamingKebabCase => field.to_ascii_uppercase().replace('_', "-"),
        }
    }

    fn apply_to_variant(self, variant: &str) -> String {
        match self {
            Self::None | Self::PascalCase => variant.to_owned(),
            Self::LowerCase => variant.to_ascii_lowercase(),
            Self::UpperCase => variant.to_ascii_uppercase(),
            Self::CamelCase => {
                let mut chars = variant.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_lowercase().to_string() + chars.as_str()
                })
            }
            Self::SnakeCase => {
                let mut snake = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            }
            Self::ScreamingSnakeCase => Self::SnakeCase
                .apply_to_variant(variant)
                .to_ascii_uppercase(),
            Self::KebabCase => Self::SnakeCase.apply_to_variant(variant).replace('_', "-"),
            Self::ScreamingKebabCase => Self::ScreamingSnakeCase
                .apply_to_variant(variant)
                .replace('_', "-"),
        }
    }
}

fn ignore_serde_meta(meta: &ParseNestedMeta<'_>) -> Result<()> {
    if meta.input.peek(Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _: TokenStream2 = content.parse()?;
    }
    Ok(())
}

fn parse_serde_serialize_str(meta: &ParseNestedMeta<'_>) -> Result<Option<LitStr>> {
    if meta.input.peek(Token![=]) {
        let lit: LitStr = meta.value()?.parse()?;
        return Ok(Some(lit));
    }
    if meta.input.peek(syn::token::Paren) {
        let mut ser_lit = None;
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("serialize") {
                ser_lit = Some(nested.value()?.parse::<LitStr>()?);
            } else {
                ignore_serde_meta(&nested)?;
            }
            Ok(())
        })?;
        return Ok(ser_lit);
    }
    Ok(None)
}

#[derive(Default)]
struct ContainerSerdeAttrs {
    rename_all: SerdeRenameRule,
    is_transparent: bool,
    not_derivable: Option<Error>,
}

impl ContainerSerdeAttrs {
    fn of(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_all") {
                    if let Some(lit) = parse_serde_serialize_str(&meta)? {
                        out.rename_all = SerdeRenameRule::from_lit(&lit)?;
                    }
                } else if meta.path.is_ident("transparent") {
                    out.is_transparent = true;
                } else if let Some(name) = ["tag", "content", "into", "remote", "untagged"]
                    .into_iter()
                    .find(|n| meta.path.is_ident(n))
                {
                    if out.not_derivable.is_none() {
                        out.not_derivable = Some(Error::new(
                            meta.path.span(),
                            format!(
                                "the schema of a type with #[serde({name})] can not be derived; \
                                 implement architecturekit::Schema by hand"
                            ),
                        ));
                    }
                    ignore_serde_meta(&meta)?;
                } else {
                    ignore_serde_meta(&meta)?;
                }
                Ok(())
            })?;
        }
        Ok(out)
    }

    fn ensure_derivable(&self) -> Result<()> {
        self.not_derivable.clone().map_or(Ok(()), Err)
    }
}

#[derive(Default)]
struct FieldSerdeAttrs {
    rename: Option<String>,
    is_skipped: bool,
    is_optional: bool,
    not_derivable: Option<Error>,
}

impl FieldSerdeAttrs {
    fn of(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    if let Some(lit) = parse_serde_serialize_str(&meta)? {
                        out.rename = Some(lit.value());
                    }
                } else if meta.path.is_ident("skip") || meta.path.is_ident("skip_serializing") {
                    out.is_skipped = true;
                    ignore_serde_meta(&meta)?;
                } else if meta.path.is_ident("skip_serializing_if") {
                    out.is_optional = true;
                    ignore_serde_meta(&meta)?;
                } else if let Some(name) = ["flatten", "serialize_with", "with"]
                    .into_iter()
                    .find(|n| meta.path.is_ident(n))
                {
                    if out.not_derivable.is_none() {
                        out.not_derivable = Some(Error::new(
                            meta.path.span(),
                            format!(
                                "the schema of a field or variant with #[serde({name})] can not be derived; \
                                 implement architecturekit::Schema by hand"
                            ),
                        ));
                    }
                    ignore_serde_meta(&meta)?;
                } else {
                    ignore_serde_meta(&meta)?;
                }
                Ok(())
            })?;
        }
        Ok(out)
    }

    fn ensure_derivable(&self) -> Result<()> {
        self.not_derivable.clone().map_or(Ok(()), Err)
    }
}

pub(crate) fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_schema(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_schema(input: &DeriveInput) -> Result<TokenStream2> {
    let container = ContainerSerdeAttrs::of(&input.attrs)?;
    let body = match &input.data {
        Data::Struct(data) => schema_of_struct(&container, &data.fields, input)?,
        Data::Enum(data) => {
            if let Some(param) = input.generics.params.first() {
                return Err(Error::new_spanned(
                    param,
                    "an enum deriving Schema can not be generic",
                ));
            }
            schema_of_unit_enum(&container, data)?
        }
        Data::Union(data) => {
            return Err(Error::new(
                data.union_token.span,
                "the schema of a union can not be derived; implement architecturekit::Schema by hand",
            ));
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

fn schema_of_struct(
    container: &ContainerSerdeAttrs,
    fields: &Fields,
    input: &DeriveInput,
) -> Result<TokenStream2> {
    container.ensure_derivable()?;

    match fields {
        Fields::Named(named) => {
            let mut written = Vec::new();
            for field in &named.named {
                let attrs = FieldSerdeAttrs::of(&field.attrs)?;
                if attrs.is_skipped {
                    continue;
                }
                attrs.ensure_derivable()?;
                written.push((field, attrs));
            }

            if container.is_transparent {
                return match written.as_slice() {
                    [(field, _)] => {
                        let field_type = &field.ty;
                        Ok(
                            quote! { <#field_type as chromatofoor::architecturekit::Schema>::schema() },
                        )
                    }
                    _ => Err(Error::new(
                        input.ident.span(),
                        "a transparent struct needs exactly one field that is not skipped",
                    )),
                };
            }

            let entries = written.iter().map(|(field, attrs)| {
                let name = attrs.rename.clone().unwrap_or_else(|| {
                    let ident = field
                        .ident
                        .as_ref()
                        .map(IdentExt::unraw)
                        .map(|i| i.to_string());
                    container
                        .rename_all
                        .apply_to_field(&ident.unwrap_or_default())
                });
                let field_type = &field.ty;
                if attrs.is_optional {
                    quote! { (#name, <#field_type as chromatofoor::architecturekit::Schema>::schema_without_null(), true) }
                } else {
                    quote! { (#name, <#field_type as chromatofoor::architecturekit::Schema>::schema(), false) }
                }
            });

            Ok(quote! {
                chromatofoor::common::schema::object_schema(::core::any::type_name::<Self>(), ::std::vec![#(#entries),*])
            })
        }
        Fields::Unnamed(unnamed) => match unnamed.unnamed.first() {
            Some(field) if unnamed.unnamed.len() == 1 => {
                let field_type = &field.ty;
                Ok(quote! { <#field_type as chromatofoor::architecturekit::Schema>::schema() })
            }
            _ => Err(Error::new(
                unnamed.span(),
                "the schema of a tuple struct with several fields can not be derived, since serde writes it \
                 as an array of mixed values; use named fields, or implement architecturekit::Schema by hand",
            )),
        },
        Fields::Unit => Err(Error::new(
            input.ident.span(),
            "the schema of a unit struct can not be derived, since serde writes it as null; \
             to describe data without fields, use `struct Name {}`",
        )),
    }
}

fn schema_of_unit_enum(
    container: &ContainerSerdeAttrs,
    data: &syn::DataEnum,
) -> Result<TokenStream2> {
    container.ensure_derivable()?;

    let mut variant_names = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new(
                data.enum_token.span,
                "the schema of an enum with non-unit variants can not be derived, since serde writes enums in several shapes; \
                 implement architecturekit::Schema by hand",
            ));
        }

        let attrs = FieldSerdeAttrs::of(&variant.attrs)?;
        if attrs.is_skipped {
            continue;
        }
        attrs.ensure_derivable()?;
        let serialized_name = attrs.rename.unwrap_or_else(|| {
            let raw = variant.ident.unraw().to_string();
            container.rename_all.apply_to_variant(&raw)
        });
        variant_names.push(serialized_name);
    }

    Ok(quote! {
        chromatofoor::common::schema::string_enum_schema(&[#(#variant_names),*])
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

    let Data::Enum(data) = &input.data else {
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
