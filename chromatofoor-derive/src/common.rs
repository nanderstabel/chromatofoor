use quote::quote;
use syn::{
    Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

pub(crate) struct ScopeAttr {
    pub(crate) scope: Path,
}

impl Parse for ScopeAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "scope" {
            return Err(syn::Error::new_spanned(key, "expected `scope = <Type>`"));
        }
        let _: Token![=] = input.parse()?;
        let scope: Path = input.parse()?;
        Ok(Self { scope })
    }
}

pub(crate) enum IdSourceSpec {
    State(Ident),
    CommandPayload(Ident),
    Schema(Ident),
    Static(String),
    Generated(Ident),
}

impl Parse for IdSourceSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![static]) {
            let _: Token![static] = input.parse()?;
            let content;
            syn::parenthesized!(content in input);
            let lit: LitStr = content.parse()?;
            return Ok(Self::Static(lit.value()));
        }

        let kind: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        if kind == "state" {
            Ok(Self::State(content.parse()?))
        } else if kind == "command_payload" {
            Ok(Self::CommandPayload(content.parse()?))
        } else if kind == "schema" {
            Ok(Self::Schema(content.parse()?))
        } else if kind == "generated" {
            let gen_ident: Ident = content.parse()?;
            if gen_ident != "uuid" {
                return Err(syn::Error::new_spanned(
                    gen_ident,
                    "expected `generated(uuid)`",
                ));
            }
            Ok(Self::Generated(gen_ident))
        } else {
            Err(syn::Error::new_spanned(
                kind,
                "expected `state(...)`, `command_payload(...)`, `schema(...)`, `static(\"...\")`, or `generated(...)`",
            ))
        }
    }
}

pub(crate) struct IdentifiedByAttr {
    pub(crate) scope: Path,
    pub(crate) identified_by: Vec<IdSourceSpec>,
}

impl Parse for IdentifiedByAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut identified_by = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            if key == "scope" {
                scope = Some(input.parse()?);
            } else if key == "identified_by" {
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<IdSourceSpec, Token![,]>::parse_terminated(
                            &content,
                        )?;
                    identified_by.extend(parsed);
                } else {
                    identified_by.push(input.parse()?);
                }
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "expected `scope = <Type>` or `identified_by = ...`",
                ));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };

        Ok(Self {
            scope,
            identified_by,
        })
    }
}

pub(crate) fn identified_impl(
    ident: &Ident,
    specs: &[IdSourceSpec],
    default_spec: IdSourceSpec,
    has_lifecycle: bool,
    is_struct: bool,
) -> proc_macro2::TokenStream {
    let effective: Vec<&IdSourceSpec> = if specs.is_empty() {
        vec![&default_spec]
    } else {
        specs.iter().collect()
    };

    let mut source_tokens = Vec::new();
    let mut field_assertions = Vec::new();

    for spec in effective {
        match spec {
            IdSourceSpec::State(field) => {
                let f_str = field.to_string();
                let f_span = field.span();
                source_tokens.push(quote! {
                    chromatofoor::common::IdSource::State(#f_str)
                });
                if is_struct {
                    field_assertions.push(quote::quote_spanned! { f_span =>
                        let _ = chromatofoor::common::FieldAsIdStr::as_id_str(&s.#field);
                    });
                }
            }
            IdSourceSpec::CommandPayload(field) => {
                let f_str = field.to_string();
                source_tokens.push(quote! {
                    chromatofoor::common::IdSource::CommandPayload(#f_str)
                });
            }
            IdSourceSpec::Schema(field) => {
                let f_str = field.to_string();
                let f_span = field.span();
                source_tokens.push(quote! {
                    chromatofoor::common::IdSource::Schema(#f_str)
                });
                if is_struct {
                    field_assertions.push(quote::quote_spanned! { f_span =>
                        let _ = chromatofoor::common::FieldAsIdStr::as_id_str(&s.#field);
                    });
                }
            }
            IdSourceSpec::Static(val) => {
                source_tokens.push(quote! {
                    chromatofoor::common::IdSource::Static(#val)
                });
            }
            IdSourceSpec::Generated(gen_ident) => {
                let g_str = gen_ident.to_string();
                source_tokens.push(quote! {
                    chromatofoor::common::IdSource::Generated(#g_str)
                });
            }
        }
    }

    let assert_block = if field_assertions.is_empty() {
        quote! {}
    } else {
        quote! {
            const _: fn(&#ident) = |s: &#ident| {
                #(#field_assertions)*
            };
        }
    };

    quote! {
        #assert_block
        impl chromatofoor::common::Identified for #ident {
            const IDENTIFIED_BY: &'static [chromatofoor::common::IdSource] = &[
                #(#source_tokens),*
            ];
            const HAS_LIFECYCLE: bool = #has_lifecycle;
        }
    }
}

pub(crate) fn extract_doc_string(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc")
            && let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) = &nv.value
        {
            lines.push(lit_str.value().trim().to_owned());
        }
    }
    lines.join("\n").trim().to_owned()
}

pub(crate) fn generics_with_static_send_sync(generics: &syn::Generics) -> syn::Generics {
    let mut out = generics.clone();
    for param in &mut out.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!('static));
            type_param
                .bounds
                .push(syn::parse_quote!(::core::marker::Send));
            type_param
                .bounds
                .push(syn::parse_quote!(::core::marker::Sync));
        }
    }
    out
}

pub(crate) fn child_kind_impl_generics(
    ident: &Ident,
    generics: &syn::Generics,
    scope_ty: &Path,
    kind_str: &str,
    name_str: &str,
    description_str: &str,
) -> proc_macro2::TokenStream {
    let mut generics_with_static = generics.clone();
    for param in &mut generics_with_static.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote!('static));
        }
    }
    let (impl_generics, _, _) = generics_with_static.split_for_impl();
    let (_, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics chromatofoor::common::Kind for #ident #ty_generics #where_clause {
            type Scope = #scope_ty;
            const KIND: &'static str = #kind_str;
            const NAME: &'static str = #name_str;
            const TOPIC: &'static str = chromatofoor::constcat::concat!(
                <#scope_ty as chromatofoor::common::Kind>::TOPIC,
                "/",
                #name_str,
            );
            const DESCRIPTION: &'static str = #description_str;
        }
    }
}

pub(crate) fn child_kind_impl(
    ident: &Ident,
    scope_ty: &Path,
    kind_str: &str,
    name_str: &str,
    description_str: &str,
) -> proc_macro2::TokenStream {
    child_kind_impl_generics(
        ident,
        &syn::Generics::default(),
        scope_ty,
        kind_str,
        name_str,
        description_str,
    )
}

pub(crate) fn authorized_impl_block(
    ident: &Ident,
    bc_ty: &proc_macro2::TokenStream,
    actors: &[Path],
) -> proc_macro2::TokenStream {
    let mut actor_entries = Vec::new();
    let mut bc_checks = Vec::new();

    for actor in actors {
        actor_entries.push(quote! {
            (
                <#actor as chromatofoor::common::Kind>::NAME,
                <#actor as chromatofoor::bounded_context::Actor>::ACTOR_TYPE,
            )
        });
        let actor_span = actor.span();
        bc_checks.push(quote::quote_spanned! { actor_span =>
            chromatofoor::bounded_context::actor::assert_actor_in_bc::<#bc_ty, #actor>();
        });
    }

    let check_block = if bc_checks.is_empty() {
        quote! {}
    } else {
        quote! {
            const _: fn() = || {
                #(#bc_checks)*
            };
        }
    };

    quote! {
        #check_block
        impl chromatofoor::bounded_context::Authorized for #ident {
            const BOUNDED_CONTEXT: &'static str =
                <#bc_ty as chromatofoor::common::Kind>::NAME;
            const ALLOWED_ACTORS: &'static [(
                &'static str,
                chromatofoor::bounded_context::ActorType,
            )] = &[#(#actor_entries),*];
        }
    }
}

pub(crate) fn extract_actors_attr(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Vec<Path>> {
    let mut actors = Vec::new();
    let mut err: Option<syn::Error> = None;
    attrs.retain(|attr| {
        if attr.path().is_ident("actors") {
            match attr
                .parse_args_with(syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated)
            {
                Ok(parsed) => actors.extend(parsed),
                Err(e) => {
                    if let Some(existing) = &mut err {
                        existing.combine(e);
                    } else {
                        err = Some(e);
                    }
                }
            }
            return false;
        }
        true
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(actors)
}

pub(crate) fn path_key(p: &Path) -> String {
    p.segments
        .iter()
        .map(|s| {
            let ident = syn::ext::IdentExt::unraw(&s.ident).to_string();
            if s.arguments.is_empty() {
                ident
            } else {
                let args = &s.arguments;
                format!("{ident}{}", quote!(#args))
            }
        })
        .collect::<Vec<_>>()
        .join("::")
}

pub(crate) fn dedup_paths_with_variant_idents(
    paths: &[Path],
    fallback_prefix: &str,
    fallback_span: proc_macro2::Span,
) -> Vec<(Ident, Path)> {
    use convert_case::{Case, Casing};

    let mut deduped: Vec<Path> = Vec::new();
    for p in paths {
        let key = path_key(p);
        if !deduped.iter().any(|existing| path_key(existing) == key) {
            deduped.push(p.clone());
        }
    }

    let mut last_seg_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for p in &deduped {
        if let Some(last) = p.segments.last() {
            let unraw_last = syn::ext::IdentExt::unraw(&last.ident).to_string();
            *last_seg_counts.entry(unraw_last).or_insert(0) += 1;
        }
    }

    let mut used_idents: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result = Vec::with_capacity(deduped.len());

    for (idx, p) in deduped.into_iter().enumerate() {
        let base_ident = match p.segments.last() {
            Some(last) => {
                let last_name = syn::ext::IdentExt::unraw(&last.ident).to_string();
                if last_seg_counts.get(&last_name).copied().unwrap_or(0) > 1 && p.segments.len() > 1
                {
                    let prefix: String = p
                        .segments
                        .iter()
                        .take(p.segments.len() - 1)
                        .map(|s| syn::ext::IdentExt::unraw(&s.ident).to_string())
                        .filter(|s| s != "self" && s != "super" && s != "crate")
                        .map(|s| s.to_case(Case::Pascal))
                        .collect();
                    let candidate = format!("{prefix}{last_name}");
                    Ident::new(&candidate, last.ident.span())
                } else {
                    last.ident.clone()
                }
            }
            None => Ident::new(&format!("{fallback_prefix}{idx}"), fallback_span),
        };

        let base_key = syn::ext::IdentExt::unraw(&base_ident).to_string();
        let final_ident = if used_idents.insert(base_key.clone()) {
            base_ident
        } else {
            let disambiguated = Ident::new(&format!("{base_key}_{idx}"), base_ident.span());
            used_idents.insert(disambiguated.to_string());
            disambiguated
        };

        result.push((final_ident, p));
    }

    result
}

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
    fn from_lit(lit: &LitStr) -> syn::Result<Self> {
        match lit.value().as_str() {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            other => Err(syn::Error::new(
                lit.span(),
                format!("unknown serde rename rule `{other}`"),
            )),
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

fn ignore_serde_meta(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _: proc_macro2::TokenStream = content.parse()?;
    }
    Ok(())
}

fn parse_serde_serialize_str(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<Option<LitStr>> {
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

pub(crate) fn unit_enum_schema_impl(
    ident: &Ident,
    attrs: &[syn::Attribute],
    data: &syn::DataEnum,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut rename_all = SerdeRenameRule::None;

    for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                if let Some(lit) = parse_serde_serialize_str(&meta)? {
                    rename_all = SerdeRenameRule::from_lit(&lit)?;
                }
            } else if let Some(name) = ["tag", "content", "into", "remote", "untagged"]
                .into_iter()
                .find(|n| meta.path.is_ident(n))
            {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    format!(
                        "the schema of a value_object enum with #[serde({name})] can not be derived"
                    ),
                ));
            } else {
                ignore_serde_meta(&meta)?;
            }
            Ok(())
        })?;
    }

    let mut variant_names = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, syn::Fields::Unit) {
            return Err(syn::Error::new(
                data.enum_token.span,
                "the schema of a value_object enum with non-unit variants can not be derived, since serde writes enums in several shapes",
            ));
        }

        let mut is_skipped = false;
        let mut rename: Option<String> = None;
        for attr in variant.attrs.iter().filter(|a| a.path().is_ident("serde")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") || meta.path.is_ident("skip_serializing") {
                    is_skipped = true;
                    ignore_serde_meta(&meta)?;
                } else if meta.path.is_ident("rename") {
                    if let Some(lit) = parse_serde_serialize_str(&meta)? {
                        rename = Some(lit.value());
                    }
                } else if let Some(name) = ["serialize_with", "with"]
                    .into_iter()
                    .find(|n| meta.path.is_ident(n))
                {
                    return Err(syn::Error::new_spanned(
                        &meta.path,
                        format!(
                            "the schema of an enum variant with #[serde({name})] can not be derived"
                        ),
                    ));
                } else {
                    ignore_serde_meta(&meta)?;
                }
                Ok(())
            })?;
        }

        if is_skipped {
            continue;
        }

        let serialized_name = rename.unwrap_or_else(|| {
            let raw = syn::ext::IdentExt::unraw(&variant.ident).to_string();
            rename_all.apply_to_variant(&raw)
        });
        variant_names.push(serialized_name);
    }

    Ok(quote! {
        impl chromatofoor::architecturekit::Schema for #ident {
            fn schema() -> chromatofoor::serde_json::Value {
                chromatofoor::serde_json::json!({
                    "type": "string",
                    "enum": [#(#variant_names),*],
                })
            }
        }
    })
}
