use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Ident, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

use crate::common::{child_kind_impl, extract_doc_string};

pub fn domain(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let topic_str = format!("/{name_str}");
    let description_str = extract_doc_string(&input.attrs);

    quote! {
        #input

        impl chromatofoor::common::Kind for #ident {
            type Scope = ();
            const KIND: &'static str = "domain";
            const NAME: &'static str = #name_str;
            const TOPIC: &'static str = #topic_str;
            const DESCRIPTION: &'static str = #description_str;
        }

        impl chromatofoor::domain::Domain for #ident {}
    }
    .into()
}

struct SubdomainAttr {
    scope: Path,
    subdomain_type: Ident,
    bounded_contexts: Vec<Path>,
}

impl Parse for SubdomainAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut subdomain_type = None;
        let mut bounded_contexts = None;

        while !input.is_empty() {
            if input.peek(Token![type]) {
                let _: Token![type] = input.parse()?;
                let _: Token![=] = input.parse()?;
                subdomain_type = Some(input.parse()?);
            } else {
                let key: Ident = input.parse()?;
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "bounded_contexts" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    if parsed.is_empty() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "`bounded_contexts` must contain at least one bounded context",
                        ));
                    }
                    bounded_contexts = Some(parsed.into_iter().collect());
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope`, `type`, or `bounded_contexts`",
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Domain>`"));
        };
        let Some(subdomain_type) = subdomain_type else {
            return Err(syn::Error::new(
                input.span(),
                "missing `type = core | supporting | generic`",
            ));
        };
        let Some(bounded_contexts) = bounded_contexts else {
            return Err(syn::Error::new(
                input.span(),
                "missing `bounded_contexts = [<BoundedContext>, ...]`",
            ));
        };

        Ok(Self {
            scope,
            subdomain_type,
            bounded_contexts,
        })
    }
}

/// Marks a type as an ESDM `subdomain` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this subdomain belongs to.
/// - `type = core | supporting | generic` *(required)*: Strategic classification of this subdomain.
/// - `bounded_contexts = [<BoundedContext>, ...]` *(required, non-empty)*: The bounded contexts within
///   `scope` that this subdomain covers.
pub fn subdomain(attr: TokenStream, item: TokenStream) -> TokenStream {
    let SubdomainAttr {
        scope: scope_ty,
        subdomain_type,
        bounded_contexts,
    } = parse_macro_input!(attr as SubdomainAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "subdomain", &name_str, &description_str);

    let type_tokens = match subdomain_type.to_string().as_str() {
        "core" => quote! { chromatofoor::domain::SubdomainType::Core },
        "supporting" => quote! { chromatofoor::domain::SubdomainType::Supporting },
        "generic" => quote! { chromatofoor::domain::SubdomainType::Generic },
        _ => {
            return syn::Error::new_spanned(
                subdomain_type,
                "expected `core`, `supporting`, or `generic`",
            )
            .to_compile_error()
            .into();
        }
    };

    let bc_checks: Vec<_> = bounded_contexts
        .iter()
        .map(|bc| {
            let span = bc.span();
            quote::quote_spanned! { span =>
                chromatofoor::domain::assert_bounded_context_in_domain::<#scope_ty, #bc>();
            }
        })
        .collect();

    let bc_names: Vec<_> = bounded_contexts
        .iter()
        .map(|bc| {
            quote! { <#bc as chromatofoor::common::Kind>::NAME }
        })
        .collect();

    quote! {
        #input
        #kind_impl
        const _: fn() = || {
            #(#bc_checks)*
        };
        impl chromatofoor::domain::Subdomain for #ident {
            const SUBDOMAIN_TYPE: chromatofoor::domain::SubdomainType = #type_tokens;
            const BOUNDED_CONTEXTS: &'static [&'static str] = &[
                #(#bc_names),*
            ];
        }
    }
    .into()
}
