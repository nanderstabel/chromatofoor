use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

use crate::{
    bounded_context::build_events_enum_and_dispatch,
    common::{
        child_kind_impl, child_kind_impl_generics, dedup_paths_with_variant_idents,
        extract_doc_string, generics_with_static_send_sync, path_key,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeliveryGuaranteeSpec {
    AtLeastOnce,
    AtMostOnce,
}

struct IdempotencySpec {
    owner: proc_macro2::TokenStream,
    strategy: Option<syn::LitStr>,
}

fn parse_idempotency_owner(input: ParseStream<'_>) -> syn::Result<proc_macro2::TokenStream> {
    if input.peek(Token![self]) {
        let _: Token![self] = input.parse()?;
        return Ok(quote! { chromatofoor::integration::IdempotencyOwner::SelfManaged });
    }
    let ident: Ident = input.parse()?;
    match ident.to_string().as_str() {
        "self_managed" => Ok(quote! { chromatofoor::integration::IdempotencyOwner::SelfManaged }),
        "downstream" => Ok(quote! { chromatofoor::integration::IdempotencyOwner::Downstream }),
        "infrastructure" => {
            Ok(quote! { chromatofoor::integration::IdempotencyOwner::Infrastructure })
        }
        "none" => Ok(quote! { chromatofoor::integration::IdempotencyOwner::None }),
        "not_required" => Ok(quote! { chromatofoor::integration::IdempotencyOwner::NotRequired }),
        _ => Err(syn::Error::new_spanned(
            ident,
            "expected `self`, `downstream`, `infrastructure`, `none`, or `not_required`",
        )),
    }
}

fn parse_idempotency_spec(input: ParseStream<'_>) -> syn::Result<IdempotencySpec> {
    if input.peek(Token![=]) {
        let _: Token![=] = input.parse()?;
        let owner = parse_idempotency_owner(input)?;
        let strategy = if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            Some(content.parse::<syn::LitStr>()?)
        } else {
            None
        };
        Ok(IdempotencySpec { owner, strategy })
    } else if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let mut owner = None;
        let mut strategy = None;
        while !content.is_empty() {
            let k: Ident = content.parse()?;
            let _: Token![=] = content.parse()?;
            if k == "owner" {
                owner = Some(parse_idempotency_owner(&content)?);
            } else if k == "strategy" {
                strategy = Some(content.parse::<syn::LitStr>()?);
            } else {
                return Err(syn::Error::new_spanned(
                    k,
                    "expected `owner = ...` or `strategy = \"...\"`",
                ));
            }
            if content.peek(Token![,]) {
                let _: Token![,] = content.parse()?;
            }
        }
        let Some(owner) = owner else {
            return Err(syn::Error::new(
                content.span(),
                "missing `owner = ...` in `idempotency(...)`",
            ));
        };
        Ok(IdempotencySpec { owner, strategy })
    } else {
        Err(syn::Error::new(
            input.span(),
            "expected `idempotency = <owner>` or `idempotency(owner = ..., strategy = \"...\")`",
        ))
    }
}

struct PolicyAttr {
    scope: Path,
    handles: Vec<Path>,
    emits: Vec<Path>,
    delivery_guarantee: DeliveryGuaranteeSpec,
    idempotency: Option<IdempotencySpec>,
}

impl Parse for PolicyAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut handles = Vec::new();
        let mut emits = Vec::new();
        let mut delivery_guarantee = None;
        let mut idempotency = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "idempotency" {
                idempotency = Some(parse_idempotency_spec(input)?);
            } else {
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "handles" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    handles.extend(parsed);
                } else if key == "emits" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    emits.extend(parsed);
                } else if key == "delivery_guarantee" {
                    let dg: Ident = input.parse()?;
                    if dg == "at_least_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtLeastOnce);
                    } else if dg == "at_most_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtMostOnce);
                    } else {
                        return Err(syn::Error::new_spanned(
                            dg,
                            "expected `at_least_once` or `at_most_once`",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope`, `handles`, `emits`, `delivery_guarantee`, or `idempotency`",
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
        if handles.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "policy requires `handles = [<Event>, ...]` with at least one event",
            ));
        }
        if emits.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "policy requires `emits = [<Command>, ...]` with at least one command",
            ));
        }

        let delivery_guarantee = delivery_guarantee.unwrap_or(if idempotency.is_some() {
            DeliveryGuaranteeSpec::AtLeastOnce
        } else {
            DeliveryGuaranteeSpec::AtMostOnce
        });

        if delivery_guarantee == DeliveryGuaranteeSpec::AtLeastOnce && idempotency.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "`delivery_guarantee = at_least_once` requires `idempotency = ...`",
            ));
        }

        Ok(Self {
            scope,
            handles,
            emits,
            delivery_guarantee,
            idempotency,
        })
    }
}

fn build_commands_enum_and_execute(
    vis: &syn::Visibility,
    commands_enum_ident: &Ident,
    scope_ty: &Path,
    emits: &[Path],
) -> (proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>) {
    let mut command_variants = Vec::new();
    let mut from_impls = Vec::new();
    let mut execute_arms = Vec::new();
    let mut domain_checks = Vec::new();

    for (var_ident, cmd_path) in
        dedup_paths_with_variant_idents(emits, "Command", commands_enum_ident.span())
    {
        command_variants.push(quote! {
            #var_ident(#cmd_path)
        });
        from_impls.push(quote! {
            impl ::core::convert::From<#cmd_path> for #commands_enum_ident {
                fn from(value: #cmd_path) -> Self {
                    Self::#var_ident(value)
                }
            }
        });
        execute_arms.push(quote! {
            Self::#var_ident(cmd) => chromatofoor::integration::execute_policy_command::<
                <#cmd_path as chromatofoor::common::Kind>::Scope,
                #cmd_path,
            >(store, cmd)
            .await
        });
        let cmd_span = cmd_path.span();
        domain_checks.push(quote::quote_spanned! { cmd_span =>
            chromatofoor::bounded_context::command::assert_command_in_domain::<#scope_ty, #cmd_path>();
        });
    }

    let commands_def = quote! {
        #[derive(
            Clone,
            Debug,
            PartialEq,
        )]
        #vis enum #commands_enum_ident {
            #(#command_variants,)*
        }

        #(#from_impls)*

        impl #commands_enum_ident {
            #vis async fn execute(
                self,
                store: &chromatofoor::architecturekit::Store,
            ) -> ::core::result::Result<
                ::std::vec::Vec<chromatofoor::architecturekit::eventsourcingdb::Event>,
                chromatofoor::architecturekit::Error<
                    chromatofoor::architecturekit::DomainError,
                >,
            > {
                match self {
                    #(#execute_arms,)*
                }
            }
        }
    };

    (commands_def, domain_checks)
}

fn build_delivery_and_idempotency(
    delivery_guarantee: DeliveryGuaranteeSpec,
    idempotency: Option<IdempotencySpec>,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let dg_expr = match delivery_guarantee {
        DeliveryGuaranteeSpec::AtLeastOnce => {
            quote! { chromatofoor::integration::DeliveryGuarantee::AtLeastOnce }
        }
        DeliveryGuaranteeSpec::AtMostOnce => {
            quote! { chromatofoor::integration::DeliveryGuarantee::AtMostOnce }
        }
    };

    let idempotency_expr = match idempotency {
        Some(IdempotencySpec { owner, strategy }) => {
            let strat_expr = match strategy {
                Some(s) => quote! { ::core::option::Option::Some(#s) },
                None => quote! { ::core::option::Option::None },
            };
            quote! {
                ::core::option::Option::Some(chromatofoor::integration::Idempotency {
                    owner: #owner,
                    strategy: #strat_expr,
                })
            }
        }
        None => quote! { ::core::option::Option::None },
    };

    (dg_expr, idempotency_expr)
}

/// Marks a struct as an ESDM `policy` scoped to a `Domain`.
///
/// Generates `<Name>Events` and `<Name>Commands` sum types and implements
/// `chromatofoor::integration::Policy`. Implement `chromatofoor::integration::React<E>`
/// for each event type listed in `handles`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this policy belongs to.
/// - `handles = [<Event>, ...]` *(required)*: Events that trigger this policy.
/// - `emits = [<Command>, ...]` *(required)*: Commands this policy may issue.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
pub fn policy(attr: TokenStream, item: TokenStream) -> TokenStream {
    let PolicyAttr {
        scope: scope_ty,
        handles,
        emits,
        delivery_guarantee,
        idempotency,
    } = parse_macro_input!(attr as PolicyAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let vis = &input.vis;
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl_generics(
        ident,
        &input.generics,
        &scope_ty,
        "policy",
        &name_str,
        &description_str,
    );

    let bounded_generics = generics_with_static_send_sync(&input.generics);
    let (impl_generics, _, _) = bounded_generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let events_enum_ident = Ident::new(&format!("{ident}Events"), ident.span());
    let commands_enum_ident = Ident::new(&format!("{ident}Commands"), ident.span());

    let (events_enum_def, react_arms) =
        build_events_enum_and_dispatch(vis, &events_enum_ident, &handles, |ev_path| {
            quote! {
                <Self as chromatofoor::integration::React<#ev_path>>::react(self, mapped)
            }
        });

    let (commands_def, mut domain_checks) =
        build_commands_enum_and_execute(vis, &commands_enum_ident, &scope_ty, &emits);

    for ev_path in &handles {
        let ev_span = ev_path.span();
        domain_checks.push(quote::quote_spanned! { ev_span =>
            chromatofoor::bounded_context::event::assert_event_in_domain::<#scope_ty, #ev_path>();
        });
    }

    let (dg_expr, idempotency_expr) =
        build_delivery_and_idempotency(delivery_guarantee, idempotency);

    quote! {
        #input
        #kind_impl

        const _: fn() = || {
            #(#domain_checks)*
        };

        #events_enum_def
        #commands_def

        impl #impl_generics chromatofoor::integration::EventConsumer for #ident #ty_generics #where_clause {
            type Event = #events_enum_ident;
            type Error = chromatofoor::architecturekit::Error<
                chromatofoor::architecturekit::DomainError,
            >;

            const DELIVERY_GUARANTEE: chromatofoor::integration::DeliveryGuarantee = #dg_expr;
            const IDEMPOTENCY: ::core::option::Option<chromatofoor::integration::Idempotency> = #idempotency_expr;

            async fn handle(
                &self,
                store: &chromatofoor::architecturekit::Store,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::core::result::Result<(), Self::Error> {
                for cmd in <Self as chromatofoor::integration::Policy>::react(self, event) {
                    cmd.execute(store).await?;
                }
                ::core::result::Result::Ok(())
            }
        }

        impl #impl_generics chromatofoor::integration::Policy for #ident #ty_generics #where_clause {
            type Command = #commands_enum_ident;

            fn react(
                &self,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::std::vec::Vec<Self::Command> {
                match &event.data {
                    #(#react_arms,)*
                }
            }
        }
    }
    .into()
}

struct EventHandlerAttr {
    scope: Path,
    handles: Vec<Path>,
    external_systems: Vec<Path>,
    delivery_guarantee: DeliveryGuaranteeSpec,
    idempotency: Option<IdempotencySpec>,
    error: Option<Path>,
}

impl Parse for EventHandlerAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut handles = Vec::new();
        let mut external_systems = Vec::new();
        let mut delivery_guarantee = None;
        let mut idempotency = None;
        let mut error = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "idempotency" {
                idempotency = Some(parse_idempotency_spec(input)?);
            } else {
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "handles" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    handles.extend(parsed);
                } else if key == "external_systems" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    external_systems.extend(parsed);
                } else if key == "delivery_guarantee" {
                    let dg: Ident = input.parse()?;
                    if dg == "at_least_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtLeastOnce);
                    } else if dg == "at_most_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtMostOnce);
                    } else {
                        return Err(syn::Error::new_spanned(
                            dg,
                            "expected `at_least_once` or `at_most_once`",
                        ));
                    }
                } else if key == "error" {
                    error = Some(input.parse()?);
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope`, `handles`, `external_systems`, `delivery_guarantee`, `idempotency`, or `error`",
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
        if handles.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "event_handler requires `handles = [<Event>, ...]` with at least one event",
            ));
        }

        let delivery_guarantee = delivery_guarantee.unwrap_or(if idempotency.is_some() {
            DeliveryGuaranteeSpec::AtLeastOnce
        } else {
            DeliveryGuaranteeSpec::AtMostOnce
        });

        if delivery_guarantee == DeliveryGuaranteeSpec::AtLeastOnce && idempotency.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "`delivery_guarantee = at_least_once` requires `idempotency = ...`",
            ));
        }

        Ok(Self {
            scope,
            handles,
            external_systems,
            delivery_guarantee,
            idempotency,
            error,
        })
    }
}

/// Marks a struct as an ESDM `event-handler` scoped to a `Domain`.
///
/// Generates `<Name>Events` sum type and implements
/// `chromatofoor::integration::EventConsumer` and `chromatofoor::integration::EventHandler`.
/// Implement `chromatofoor::integration::Handle<E>` for each event type listed in `handles`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this event handler belongs to.
/// - `handles = [<Event>, ...]` *(required)*: Events that trigger this handler.
/// - `external_systems = [<ExternalSystem>, ...]` *(optional)*: External systems invoked as side effects.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
/// - `error = <ErrorType>` *(optional, defaults to `architecturekit::DomainError`)*: Error type returned by `Handle<E>::handle`.
pub fn event_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let EventHandlerAttr {
        scope: scope_ty,
        handles,
        external_systems,
        delivery_guarantee,
        idempotency,
        error,
    } = parse_macro_input!(attr as EventHandlerAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let vis = &input.vis;
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl_generics(
        ident,
        &input.generics,
        &scope_ty,
        "event-handler",
        &name_str,
        &description_str,
    );

    let bounded_generics = generics_with_static_send_sync(&input.generics);
    let (impl_generics, _, _) = bounded_generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    let events_enum_ident = Ident::new(&format!("{ident}Events"), ident.span());

    let (events_enum_def, handle_arms) =
        build_events_enum_and_dispatch(vis, &events_enum_ident, &handles, |ev_path| {
            quote! {
                <Self as chromatofoor::integration::Handle<#ev_path>>::handle(self, mapped).await
            }
        });

    let mut domain_checks = Vec::new();
    for ev_path in &handles {
        let ev_span = ev_path.span();
        domain_checks.push(quote::quote_spanned! { ev_span =>
            chromatofoor::bounded_context::event::assert_event_in_domain::<#scope_ty, #ev_path>();
        });
    }

    let mut ext_names = Vec::new();
    for ext in &external_systems {
        ext_names.push(quote! {
            <#ext as chromatofoor::common::Kind>::NAME
        });
        let ext_span = ext.span();
        domain_checks.push(quote::quote_spanned! { ext_span =>
            chromatofoor::integration::external_system::assert_external_system_in_domain::<
                #scope_ty,
                #ext,
            >();
            const _: () = assert!(
                !matches!(
                    <#ext as chromatofoor::integration::ExternalSystem>::DIRECTION,
                    chromatofoor::integration::ExternalDirection::Inbound,
                ),
                "`external_systems` on `#[event_handler]` requires an `outbound` or `bidirectional` external system",
            );
        });
    }

    let (dg_expr, idempotency_expr) =
        build_delivery_and_idempotency(delivery_guarantee, idempotency);

    let error_ty = error.map_or_else(
        || quote! { chromatofoor::architecturekit::DomainError },
        |p| quote! { #p },
    );

    quote! {
        #input
        #kind_impl

        const _: fn() = || {
            #(#domain_checks)*
        };

        #events_enum_def

        impl #impl_generics chromatofoor::integration::EventConsumer for #ident #ty_generics #where_clause {
            type Event = #events_enum_ident;
            type Error = #error_ty;

            const DELIVERY_GUARANTEE: chromatofoor::integration::DeliveryGuarantee = #dg_expr;
            const IDEMPOTENCY: ::core::option::Option<chromatofoor::integration::Idempotency> = #idempotency_expr;

            async fn handle(
                &self,
                _store: &chromatofoor::architecturekit::Store,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::core::result::Result<(), Self::Error> {
                match &event.data {
                    #(#handle_arms,)*
                }
            }
        }

        impl #impl_generics chromatofoor::integration::EventHandler for #ident #ty_generics #where_clause {
            const EXTERNAL_SYSTEMS: &'static [&'static str] = &[#(#ext_names),*];
        }
    }
    .into()
}

struct ExternalSystemAttr {
    scope: Path,
    direction: proc_macro2::TokenStream,
    category: Option<LitStr>,
}

impl Parse for ExternalSystemAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut direction = None;
        let mut category = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            if key == "scope" {
                scope = Some(input.parse()?);
            } else if key == "direction" {
                let dir: Ident = input.parse()?;
                direction = Some(match dir.to_string().as_str() {
                    "inbound" => {
                        quote! { chromatofoor::integration::ExternalDirection::Inbound }
                    }
                    "outbound" => {
                        quote! { chromatofoor::integration::ExternalDirection::Outbound }
                    }
                    "bidirectional" => {
                        quote! { chromatofoor::integration::ExternalDirection::Bidirectional }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            dir,
                            "expected `inbound`, `outbound`, or `bidirectional`",
                        ));
                    }
                });
            } else if key == "category" {
                category = Some(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "expected `scope = <Domain>`, `direction = inbound | outbound | bidirectional`, or `category = \"...\"`",
                ));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Domain>`"));
        };
        let Some(direction) = direction else {
            return Err(syn::Error::new(
                input.span(),
                "missing `direction = inbound | outbound | bidirectional`",
            ));
        };

        Ok(Self {
            scope,
            direction,
            category,
        })
    }
}

/// Marks a struct as an ESDM `external-system` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this external system belongs to.
/// - `direction = inbound | outbound | bidirectional` *(required)*: Direction of interaction.
/// - `category = "..."` *(optional)*: Short category tag (e.g. `"payment"`, `"mail"`).
pub fn external_system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ExternalSystemAttr {
        scope: scope_ty,
        direction,
        category,
    } = parse_macro_input!(attr as ExternalSystemAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(
        ident,
        &scope_ty,
        "external-system",
        &name_str,
        &description_str,
    );

    let category_expr = match category {
        Some(cat) => quote! { ::core::option::Option::Some(#cat) },
        None => quote! { ::core::option::Option::None },
    };

    quote! {
        #input
        #kind_impl
        impl chromatofoor::integration::ExternalSystem for #ident {
            const DIRECTION: chromatofoor::integration::ExternalDirection = #direction;
            const CATEGORY: ::core::option::Option<&'static str> = #category_expr;
        }
        impl chromatofoor::integration::external_system::ExternalSystemInDomain<#scope_ty> for #ident {}
        impl chromatofoor::integration::MappingEndpoint for #ident {}
    }
    .into()
}

enum ContextMappingSpec {
    Asymmetric {
        variant: Ident,
        field1: Ident,
        ep1: Path,
        field2: Ident,
        ep2: Path,
    },
    Symmetric {
        variant: Ident,
        ep1: Path,
        ep2: Path,
    },
}

impl Parse for ContextMappingSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut mapping_type: Option<Ident> = None;
        let mut roles: Vec<(Ident, Path)> = Vec::new();
        let mut participants: Option<(Ident, Vec<Path>)> = None;

        while !input.is_empty() {
            if input.peek(Token![type]) {
                let _: Token![type] = input.parse()?;
                let _: Token![=] = input.parse()?;
                mapping_type = Some(input.parse()?);
            } else {
                let key: Ident = input.parse()?;
                let _: Token![=] = input.parse()?;
                if key == "participants" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    participants = Some((key, parsed.into_iter().collect()));
                } else {
                    roles.push((key, input.parse()?));
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(ty) = mapping_type else {
            return Err(syn::Error::new(
                input.span(),
                "missing `type = customer_supplier | conformist | anti_corruption_layer | open_host_service | published_language | shared_kernel | partnership | separate_ways`",
            ));
        };

        let find_role = |expected: &str| -> syn::Result<Path> {
            roles
                .iter()
                .find(|(k, _)| k == expected)
                .map(|(_, p)| p.clone())
                .ok_or_else(|| {
                    syn::Error::new_spanned(&ty, format!("missing `{expected} = <Endpoint>`"))
                })
        };

        match ty.to_string().as_str() {
            "customer_supplier" => Ok(Self::Asymmetric {
                variant: Ident::new("CustomerSupplier", ty.span()),
                field1: Ident::new("customer", ty.span()),
                ep1: find_role("customer")?,
                field2: Ident::new("supplier", ty.span()),
                ep2: find_role("supplier")?,
            }),
            "conformist" => Ok(Self::Asymmetric {
                variant: Ident::new("Conformist", ty.span()),
                field1: Ident::new("conformist", ty.span()),
                ep1: find_role("conformist")?,
                field2: Ident::new("upstream", ty.span()),
                ep2: find_role("upstream")?,
            }),
            "anti_corruption_layer" => Ok(Self::Asymmetric {
                variant: Ident::new("AntiCorruptionLayer", ty.span()),
                field1: Ident::new("downstream", ty.span()),
                ep1: find_role("downstream")?,
                field2: Ident::new("upstream", ty.span()),
                ep2: find_role("upstream")?,
            }),
            "open_host_service" => Ok(Self::Asymmetric {
                variant: Ident::new("OpenHostService", ty.span()),
                field1: Ident::new("host", ty.span()),
                ep1: find_role("host")?,
                field2: Ident::new("consumer", ty.span()),
                ep2: find_role("consumer")?,
            }),
            "published_language" => Ok(Self::Asymmetric {
                variant: Ident::new("PublishedLanguage", ty.span()),
                field1: Ident::new("publisher", ty.span()),
                ep1: find_role("publisher")?,
                field2: Ident::new("consumer", ty.span()),
                ep2: find_role("consumer")?,
            }),
            "shared_kernel" | "partnership" | "separate_ways" => {
                let variant_name = match ty.to_string().as_str() {
                    "shared_kernel" => "SharedKernel",
                    "partnership" => "Partnership",
                    _ => "SeparateWays",
                };
                let Some((p_key, list)) = participants else {
                    return Err(syn::Error::new_spanned(
                        &ty,
                        "expected `participants = [<BoundedContext>, <BoundedContext>]`",
                    ));
                };
                if list.len() != 2 {
                    return Err(syn::Error::new_spanned(
                        p_key,
                        "`participants` must contain exactly 2 bounded contexts",
                    ));
                }
                Ok(Self::Symmetric {
                    variant: Ident::new(variant_name, ty.span()),
                    ep1: list[0].clone(),
                    ep2: list[1].clone(),
                })
            }
            _ => Err(syn::Error::new_spanned(
                ty,
                "expected `customer_supplier`, `conformist`, `anti_corruption_layer`, `open_host_service`, `published_language`, `shared_kernel`, `partnership`, or `separate_ways`",
            )),
        }
    }
}

/// Marks a struct as a top-level ESDM `context-mapping` (`Scope = ()`).
///
/// # Attributes
/// - Asymmetric patterns (`BoundedContext` or `ExternalSystem` endpoints):
///   - `type = customer_supplier, customer = <E1>, supplier = <E2>`
///   - `type = conformist, conformist = <E1>, upstream = <E2>`
///   - `type = anti_corruption_layer, downstream = <E1>, upstream = <E2>`
///   - `type = open_host_service, host = <E1>, consumer = <E2>`
///   - `type = published_language, publisher = <E1>, consumer = <E2>`
/// - Symmetric patterns (`BoundedContext`-only endpoints):
///   - `type = shared_kernel, participants = [<BC1>, <BC2>]`
///   - `type = partnership, participants = [<BC1>, <BC2>]`
///   - `type = separate_ways, participants = [<BC1>, <BC2>]`
pub fn context_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    let spec = parse_macro_input!(attr as ContextMappingSpec);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let topic_str = format!("/{name_str}");
    let description_str = extract_doc_string(&input.attrs);

    let (checks, mapping_expr) = match spec {
        ContextMappingSpec::Asymmetric {
            variant,
            field1,
            ep1,
            field2,
            ep2,
        } => {
            let s1 = ep1.span();
            let s2 = ep2.span();
            let c1 = quote::quote_spanned! { s1 =>
                chromatofoor::integration::context_mapping::assert_mapping_endpoint::<#ep1>();
            };
            let c2 = quote::quote_spanned! { s2 =>
                chromatofoor::integration::context_mapping::assert_mapping_endpoint::<#ep2>();
            };
            let distinct = quote! {
                const _: () = assert!(
                    !chromatofoor::common::const_str_eq(
                        <#ep1 as chromatofoor::common::Kind>::TOPIC,
                        <#ep2 as chromatofoor::common::Kind>::TOPIC,
                    ),
                    "a context mapping must link two distinct endpoints",
                );
            };
            (
                quote! {
                    const _: fn() = || {
                        #c1
                        #c2
                    };
                    #distinct
                },
                quote! {
                    chromatofoor::integration::ContextMappingType::#variant {
                        #field1: <#ep1 as chromatofoor::common::Kind>::TOPIC,
                        #field2: <#ep2 as chromatofoor::common::Kind>::TOPIC,
                    }
                },
            )
        }
        ContextMappingSpec::Symmetric { variant, ep1, ep2 } => {
            let s1 = ep1.span();
            let s2 = ep2.span();
            let c1 = quote::quote_spanned! { s1 =>
                chromatofoor::integration::context_mapping::assert_bounded_context_endpoint::<#ep1>();
            };
            let c2 = quote::quote_spanned! { s2 =>
                chromatofoor::integration::context_mapping::assert_bounded_context_endpoint::<#ep2>();
            };
            let distinct = quote! {
                const _: () = assert!(
                    !chromatofoor::common::const_str_eq(
                        <#ep1 as chromatofoor::common::Kind>::TOPIC,
                        <#ep2 as chromatofoor::common::Kind>::TOPIC,
                    ),
                    "a context mapping must link two distinct bounded contexts",
                );
            };
            (
                quote! {
                    const _: fn() = || {
                        #c1
                        #c2
                    };
                    #distinct
                },
                quote! {
                    chromatofoor::integration::ContextMappingType::#variant {
                        participants: [
                            <#ep1 as chromatofoor::common::Kind>::TOPIC,
                            <#ep2 as chromatofoor::common::Kind>::TOPIC,
                        ],
                    }
                },
            )
        }
    };

    quote! {
        #input
        #checks
        impl chromatofoor::common::Kind for #ident {
            type Scope = ();
            const KIND: &'static str = "context-mapping";
            const NAME: &'static str = #name_str;
            const TOPIC: &'static str = #topic_str;
            const DESCRIPTION: &'static str = #description_str;
        }
        impl chromatofoor::integration::ContextMapping for #ident {
            const MAPPING: chromatofoor::integration::ContextMappingType = #mapping_expr;
        }
    }
    .into()
}

enum TimerScheduleSpec {
    After {
        value: u64,
        unit: proc_macro2::TokenStream,
    },
    At(Ident),
}

struct TimerSpec {
    ident: Ident,
    schedule: TimerScheduleSpec,
}

impl Parse for TimerSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let mode: Ident = content.parse()?;
        let schedule = if mode == "after" {
            let inner;
            syn::parenthesized!(inner in content);
            let unit_ident: Ident = inner.parse()?;
            let _: Token![=] = inner.parse()?;
            let lit: syn::LitInt = inner.parse()?;
            let value: u64 = lit.base10_parse()?;
            if value == 0 {
                return Err(syn::Error::new_spanned(
                    lit,
                    "timer duration value must be at least 1",
                ));
            }
            let unit = match unit_ident.to_string().as_str() {
                "seconds" => quote! { chromatofoor::integration::TimerUnit::Seconds },
                "minutes" => quote! { chromatofoor::integration::TimerUnit::Minutes },
                "hours" => quote! { chromatofoor::integration::TimerUnit::Hours },
                "days" => quote! { chromatofoor::integration::TimerUnit::Days },
                "weeks" => quote! { chromatofoor::integration::TimerUnit::Weeks },
                "months" => quote! { chromatofoor::integration::TimerUnit::Months },
                "years" => quote! { chromatofoor::integration::TimerUnit::Years },
                _ => {
                    return Err(syn::Error::new_spanned(
                        unit_ident,
                        "expected `seconds`, `minutes`, `hours`, `days`, `weeks`, `months`, or `years`",
                    ));
                }
            };
            TimerScheduleSpec::After { value, unit }
        } else if mode == "at" {
            let _: Token![=] = content.parse()?;
            let field: Ident = content.parse()?;
            TimerScheduleSpec::At(field)
        } else {
            return Err(syn::Error::new_spanned(
                mode,
                "expected `after(<unit> = <value>)` or `at = <field>`",
            ));
        };
        Ok(Self { ident, schedule })
    }
}

enum CorrelationSpec {
    EventField(Ident),
    EventSubject,
}

struct ProcessManagerAttr {
    scope: Path,
    correlated_by: CorrelationSpec,
    starts_when: Vec<Path>,
    handles: Vec<Path>,
    emits: Vec<Path>,
    timers: Vec<TimerSpec>,
    ends_when: syn::Expr,
    delivery_guarantee: DeliveryGuaranteeSpec,
    idempotency: Option<IdempotencySpec>,
}

impl Parse for ProcessManagerAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut correlated_by = None;
        let mut starts_when = Vec::new();
        let mut handles = Vec::new();
        let mut emits = Vec::new();
        let mut timers = Vec::new();
        let mut ends_when = None;
        let mut delivery_guarantee = None;
        let mut idempotency = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "idempotency" {
                idempotency = Some(parse_idempotency_spec(input)?);
            } else {
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "correlated_by" {
                    let src: Ident = input.parse()?;
                    if src == "event_field" {
                        let content;
                        syn::parenthesized!(content in input);
                        correlated_by = Some(CorrelationSpec::EventField(content.parse()?));
                    } else if src == "event_subject" {
                        correlated_by = Some(CorrelationSpec::EventSubject);
                    } else {
                        return Err(syn::Error::new_spanned(
                            src,
                            "expected `event_field(<field>)` or `event_subject`",
                        ));
                    }
                } else if key == "starts_when" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    starts_when.extend(parsed);
                } else if key == "handles" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    handles.extend(parsed);
                } else if key == "emits" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    emits.extend(parsed);
                } else if key == "timers" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<TimerSpec, Token![,]>::parse_terminated(
                            &content,
                        )?;
                    timers.extend(parsed);
                } else if key == "ends_when" {
                    ends_when = Some(input.parse()?);
                } else if key == "delivery_guarantee" {
                    let dg: Ident = input.parse()?;
                    if dg == "at_least_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtLeastOnce);
                    } else if dg == "at_most_once" {
                        delivery_guarantee = Some(DeliveryGuaranteeSpec::AtMostOnce);
                    } else {
                        return Err(syn::Error::new_spanned(
                            dg,
                            "expected `at_least_once` or `at_most_once`",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope`, `correlated_by`, `starts_when`, `handles`, `emits`, `timers`, `ends_when`, `delivery_guarantee`, or `idempotency`",
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
        let Some(correlated_by) = correlated_by else {
            return Err(syn::Error::new(
                input.span(),
                "missing `correlated_by = event_field(<field>) | event_subject`",
            ));
        };
        if starts_when.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "process_manager requires `starts_when = [<Event>, ...]` with at least one event",
            ));
        }
        if emits.is_empty() && timers.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "process_manager requires `emits = [<Command>, ...]` or `timers = [...]`",
            ));
        }
        let Some(ends_when) = ends_when else {
            return Err(syn::Error::new(
                input.span(),
                "missing `ends_when = <predicate>`",
            ));
        };

        let delivery_guarantee = delivery_guarantee.unwrap_or(if idempotency.is_some() {
            DeliveryGuaranteeSpec::AtLeastOnce
        } else {
            DeliveryGuaranteeSpec::AtMostOnce
        });

        if delivery_guarantee == DeliveryGuaranteeSpec::AtLeastOnce && idempotency.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "`delivery_guarantee = at_least_once` requires `idempotency = ...`",
            ));
        }

        Ok(Self {
            scope,
            correlated_by,
            starts_when,
            handles,
            emits,
            timers,
            ends_when,
            delivery_guarantee,
            idempotency,
        })
    }
}

/// Marks a struct as an ESDM `process-manager` scoped to a `Domain`.
///
/// Generates `<Name>Events`, `<Name>Commands`, and any declared timer types, and implements
/// `chromatofoor::integration::EventConsumer` and `chromatofoor::integration::ProcessManager`.
/// Implement `chromatofoor::integration::OnEvent<E>` for each handled event and
/// `chromatofoor::integration::OnTimer<T>` for each declared timer.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this process manager belongs to.
/// - `correlated_by = event_field(<field>) | event_subject` *(required)*: Event field or subject
///   segment used to correlate incoming events to a process instance.
/// - `starts_when = [<Event>, ...]` *(required)*: Events that may instantiate a new process instance.
/// - `handles = [<Event>, ...]` *(optional)*: Additional events handled by an active process instance.
/// - `emits = [<Command>, ...]` *(required)*: Commands this process manager may emit.
/// - `timers = [<Timer>(after(<unit> = <value>)) | <Timer>(at = <field>), ...]` *(optional)*:
///   Timers managed by this process manager.
/// - `ends_when = <predicate>` *(required)*: Closure or function `fn(&Self) -> bool` indicating
///   when the process instance has reached a terminal state.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
pub fn process_manager(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ProcessManagerAttr {
        scope: scope_ty,
        correlated_by,
        starts_when,
        handles,
        emits,
        timers,
        ends_when,
        delivery_guarantee,
        idempotency,
    } = parse_macro_input!(attr as ProcessManagerAttr);
    let mut input = parse_macro_input!(item as DeriveInput);
    let vis = &input.vis;
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl_generics(
        ident,
        &input.generics,
        &scope_ty,
        "process-manager",
        &name_str,
        &description_str,
    );

    let mut pm_generics = generics_with_static_send_sync(&input.generics);
    for param in &mut pm_generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(syn::parse_quote!(::core::default::Default));
            type_param
                .bounds
                .push(syn::parse_quote!(::core::clone::Clone));
            type_param
                .bounds
                .push(syn::parse_quote!(chromatofoor::serde::Serialize));
            type_param
                .bounds
                .push(syn::parse_quote!(chromatofoor::serde::de::DeserializeOwned));
        }
    }
    let (impl_generics, _, _) = pm_generics.split_for_impl();
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();

    input.attrs.push(syn::parse_quote! {
        #[derive(chromatofoor::serde::Serialize, chromatofoor::serde::Deserialize)]
    });
    input.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });

    // Merge starts_when and handles, preserving order and deduplicating by canonical path.
    let mut starter_keys = std::collections::HashSet::new();
    let mut combined_event_paths = Vec::new();
    for ev in &starts_when {
        starter_keys.insert(path_key(ev));
        combined_event_paths.push(ev.clone());
    }
    for ev in &handles {
        combined_event_paths.push(ev.clone());
    }

    let events_enum_ident = Ident::new(&format!("{ident}Events"), ident.span());
    let commands_enum_ident = Ident::new(&format!("{ident}Commands"), ident.span());

    let mut event_variants = Vec::new();
    let mut event_from_impls = Vec::new();
    let mut event_arms = Vec::new();
    let mut step_event_arms = Vec::new();
    let mut domain_checks = Vec::new();

    let corr_expr = match &correlated_by {
        CorrelationSpec::EventField(corr_field) => quote! {
            (&mapped.data.#corr_field).to_string()
        },
        CorrelationSpec::EventSubject => quote! {
            mapped
                .subject
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        },
    };

    for (var_ident, ev_path) in
        dedup_paths_with_variant_idents(&combined_event_paths, "Event", events_enum_ident.span())
    {
        let is_starter = starter_keys.contains(&path_key(&ev_path));
        event_variants.push(quote! {
            #var_ident(#ev_path)
        });
        event_from_impls.push(quote! {
            impl ::core::convert::From<#ev_path> for #events_enum_ident {
                fn from(value: #ev_path) -> Self {
                    Self::#var_ident(value)
                }
            }
        });
        let ev_span = ev_path.span();
        domain_checks.push(quote::quote_spanned! { ev_span =>
            chromatofoor::bounded_context::event::assert_event_in_domain::<#scope_ty, #ev_path>();
        });
        step_event_arms.push(quote! {
            #events_enum_ident::#var_ident(_) => {
                let mapped = event.map(|data| match data {
                    #events_enum_ident::#var_ident(inner) => inner,
                    #[allow(unreachable_patterns, clippy::unreachable)]
                    _ => unreachable!(),
                });
                if state.is_none() {
                    if #is_starter {
                        *state = ::core::option::Option::Some(<Self as ::core::default::Default>::default());
                    } else {
                        return ::std::vec::Vec::new();
                    }
                }
                let ::core::option::Option::Some(instance) = state.as_mut() else {
                    return ::std::vec::Vec::new();
                };
                <Self as chromatofoor::integration::OnEvent<#ev_path>>::on_event(instance, mapped)
            }
        });
        event_arms.push(quote! {
            #events_enum_ident::#var_ident(_) => {
                let event_id = event.id;
                let mapped = event.map(|data| match data {
                    #events_enum_ident::#var_ident(inner) => inner,
                    #[allow(unreachable_patterns, clippy::unreachable)]
                    _ => unreachable!(),
                });
                let correlation_id = #corr_expr;
                let (mut snapshot, existing) =
                    chromatofoor::integration::load_process_instance::<Self>(store, &correlation_id).await?;
                if !snapshot.pending_commands.is_empty() {
                    chromatofoor::integration::flush_process_commands::<Self, _, _>(
                        store,
                        &correlation_id,
                        &mut snapshot,
                        #commands_enum_ident::execute_pending,
                    )
                    .await?;
                }
                if snapshot.ended
                    || snapshot.last_event_id.is_some_and(|last| event_id.get() <= last)
                {
                    return ::core::result::Result::Ok(());
                }
                let mut state = match existing {
                    ::core::option::Option::Some(s) => s,
                    ::core::option::Option::None if #is_starter => <Self as ::core::default::Default>::default(),
                    ::core::option::Option::None => return ::core::result::Result::Ok(()),
                };
                let actions = <Self as chromatofoor::integration::OnEvent<#ev_path>>::on_event(
                    &mut state,
                    mapped,
                );
                #commands_enum_ident::update_timers(&actions, &state, &mut snapshot.active_timers);
                let pending_commands = #commands_enum_ident::encode_pending(&actions)?;
                let mut snapshot = chromatofoor::integration::save_process_instance(
                    store,
                    &correlation_id,
                    event_id,
                    snapshot.active_timers,
                    pending_commands,
                    &state,
                )
                .await?;
                chromatofoor::integration::flush_process_commands::<Self, _, _>(
                    store,
                    &correlation_id,
                    &mut snapshot,
                    #commands_enum_ident::execute_pending,
                )
                .await
            }
        });
    }

    event_variants.push(quote! {
        __TimerFired(chromatofoor::integration::ProcessTimerFired)
    });
    event_from_impls.push(quote! {
        impl ::core::convert::From<chromatofoor::integration::ProcessTimerFired> for #events_enum_ident {
            fn from(value: chromatofoor::integration::ProcessTimerFired) -> Self {
                Self::__TimerFired(value)
            }
        }
    });

    let mut timer_defs = Vec::new();
    let mut timer_fire_arms = Vec::new();
    let mut timer_step_arms = Vec::new();
    let mut command_variants = Vec::new();
    let mut command_from_impls = Vec::new();
    let mut update_timer_arms = Vec::new();
    let mut encode_pending_arms = Vec::new();
    let mut execute_pending_branches = Vec::new();
    let mut emitted_command_arms = Vec::new();
    let mut set_timer_arms = Vec::new();
    let mut cancel_timer_arms = Vec::new();

    for (var_ident, cmd_path) in
        dedup_paths_with_variant_idents(&emits, "Command", commands_enum_ident.span())
    {
        command_variants.push(quote! {
            #var_ident(#cmd_path)
        });
        command_from_impls.push(quote! {
            impl ::core::convert::From<#cmd_path> for #commands_enum_ident {
                fn from(value: #cmd_path) -> Self {
                    Self::#var_ident(value)
                }
            }
        });
        encode_pending_arms.push(quote! {
            Self::#var_ident(cmd) => {
                let payload = chromatofoor::serde_json::to_value(cmd).map_err(|e| {
                    chromatofoor::architecturekit::Error::Decider(
                        chromatofoor::architecturekit::DomainError::new(e.to_string()),
                    )
                })?;
                out.push(chromatofoor::serde_json::json!({
                    "type": <#cmd_path as chromatofoor::common::Kind>::TOPIC,
                    "payload": payload,
                }));
            }
        });
        execute_pending_branches.push(quote! {
            if kind == <#cmd_path as chromatofoor::common::Kind>::TOPIC {
                let payload = raw
                    .get("payload")
                    .cloned()
                    .unwrap_or(chromatofoor::serde_json::Value::Null);
                let cmd: #cmd_path = chromatofoor::serde_json::from_value(payload).map_err(|e| {
                    chromatofoor::architecturekit::Error::Decider(
                        chromatofoor::architecturekit::DomainError::new(e.to_string()),
                    )
                })?;
                return chromatofoor::integration::execute_process_command::<
                    <#cmd_path as chromatofoor::common::Kind>::Scope,
                    #cmd_path,
                >(store, cmd)
                .await;
            }
        });
        emitted_command_arms.push(quote! {
            Self::#var_ident(cmd) => ::core::option::Option::Some((
                <<<#cmd_path as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::NAME,
                <<#cmd_path as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::NAME,
                <#cmd_path as chromatofoor::common::Kind>::NAME,
                chromatofoor::serde_json::to_value(cmd).unwrap_or(chromatofoor::serde_json::Value::Null),
            )),
        });
        let cmd_span = cmd_path.span();
        domain_checks.push(quote::quote_spanned! { cmd_span =>
            chromatofoor::bounded_context::command::assert_command_in_domain::<#scope_ty, #cmd_path>();
        });
    }

    for timer in &timers {
        let timer_ident = &timer.ident;
        let timer_name_str = timer_ident.to_string().to_case(Case::Kebab);
        let set_var = Ident::new(&format!("__Set{timer_ident}"), timer_ident.span());
        let cancel_var = Ident::new(&format!("__Cancel{timer_ident}"), timer_ident.span());

        let (schedule_expr, deadline_body) = match &timer.schedule {
            TimerScheduleSpec::After { value, unit } => (
                quote! {
                    chromatofoor::integration::TimerSchedule::After {
                        value: #value,
                        unit: #unit,
                    }
                },
                quote! {
                    ::core::option::Option::Some(#unit.add_to(chromatofoor::chrono::Utc::now(), #value))
                },
            ),
            TimerScheduleSpec::At(field) => {
                let field_str = field.to_string();
                let field_span = field.span();
                if input.generics.params.is_empty() {
                    domain_checks.push(quote::quote_spanned! { field_span =>
                        let _ = |s: &#ident| {
                            let _ = chromatofoor::integration::AsTimerTimestamp::as_timer_timestamp(&s.#field);
                        };
                    });
                }
                (
                    quote! {
                        chromatofoor::integration::TimerSchedule::At {
                            field: #field_str,
                        }
                    },
                    quote! {
                        chromatofoor::integration::AsTimerTimestamp::as_timer_timestamp(&state.#field)
                    },
                )
            }
        };

        timer_defs.push(quote! {
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
            #vis struct #timer_ident;

            impl #timer_ident {
                #[must_use]
                #vis const fn set() -> chromatofoor::integration::SetTimer<Self> {
                    chromatofoor::integration::SetTimer(Self)
                }

                #[must_use]
                #vis const fn cancel() -> chromatofoor::integration::CancelTimer<Self> {
                    chromatofoor::integration::CancelTimer(Self)
                }
            }

            impl #impl_generics chromatofoor::integration::ProcessTimer<#ident #ty_generics> for #timer_ident #where_clause {
                const NAME: &'static str = #timer_name_str;
                const SCHEDULE: chromatofoor::integration::TimerSchedule = #schedule_expr;

                fn deadline(
                    state: &#ident #ty_generics,
                ) -> ::core::option::Option<chromatofoor::chrono::DateTime<chromatofoor::chrono::Utc>> {
                    let _ = state;
                    #deadline_body
                }
            }
        });

        command_variants.push(quote! {
            #set_var(chromatofoor::integration::SetTimer<#timer_ident>)
        });
        command_from_impls.push(quote! {
            impl ::core::convert::From<chromatofoor::integration::SetTimer<#timer_ident>> for #commands_enum_ident {
                fn from(value: chromatofoor::integration::SetTimer<#timer_ident>) -> Self {
                    Self::#set_var(value)
                }
            }
        });
        command_variants.push(quote! {
            #cancel_var(chromatofoor::integration::CancelTimer<#timer_ident>)
        });
        command_from_impls.push(quote! {
            impl ::core::convert::From<chromatofoor::integration::CancelTimer<#timer_ident>> for #commands_enum_ident {
                fn from(value: chromatofoor::integration::CancelTimer<#timer_ident>) -> Self {
                    Self::#cancel_var(value)
                }
            }
        });

        update_timer_arms.push(quote! {
            Self::#set_var(_) => {
                if let ::core::option::Option::Some(deadline) = <#timer_ident as chromatofoor::integration::ProcessTimer<__State>>::deadline(state) {
                    active_timers.insert(#timer_name_str.to_owned(), deadline);
                }
            }
        });
        update_timer_arms.push(quote! {
            Self::#cancel_var(_) => {
                active_timers.remove(#timer_name_str);
            }
        });
        encode_pending_arms.push(quote! {
            Self::#set_var(_) | Self::#cancel_var(_) => {}
        });

        set_timer_arms.push(quote! {
            Self::#set_var(_) => ::core::option::Option::Some(#timer_name_str),
        });
        cancel_timer_arms.push(quote! {
            Self::#cancel_var(_) => ::core::option::Option::Some(#timer_name_str),
        });

        timer_fire_arms.push(quote! {
            #timer_name_str => <Self as chromatofoor::integration::OnTimer<#timer_ident>>::on_timer(&mut state)
        });
        timer_step_arms.push(quote! {
            #timer_name_str => <Self as chromatofoor::integration::OnTimer<#timer_ident>>::on_timer(instance)
        });
    }

    step_event_arms.push(quote! {
        #events_enum_ident::__TimerFired(timer_ev) => {
            if !active_timers.remove(timer_ev.timer.as_str()) {
                return ::std::vec::Vec::new();
            }
            let ::core::option::Option::Some(instance) = state.as_mut() else {
                return ::std::vec::Vec::new();
            };
            match timer_ev.timer.as_str() {
                #(#timer_step_arms,)*
                _ => ::std::vec::Vec::new(),
            }
        }
    });

    event_arms.push(quote! {
        #events_enum_ident::__TimerFired(timer_ev) => {
            if timer_ev.process_manager != <Self as chromatofoor::common::Kind>::TOPIC {
                return ::core::result::Result::Ok(());
            }
            let (mut snapshot, existing) =
                chromatofoor::integration::load_process_instance::<Self>(store, &timer_ev.correlation_id).await?;
            if !snapshot.pending_commands.is_empty() {
                chromatofoor::integration::flush_process_commands::<Self, _, _>(
                    store,
                    &timer_ev.correlation_id,
                    &mut snapshot,
                    #commands_enum_ident::execute_pending,
                )
                .await?;
            }
            if snapshot.ended
                || snapshot.last_event_id.is_some_and(|last| event.id.get() <= last)
            {
                return ::core::result::Result::Ok(());
            }
            let ::core::option::Option::Some(&armed_deadline) =
                snapshot.active_timers.get(&timer_ev.timer)
            else {
                return ::core::result::Result::Ok(());
            };
            if timer_ev
                .deadline
                .is_some_and(|fired_deadline| fired_deadline != armed_deadline)
            {
                return ::core::result::Result::Ok(());
            };
            snapshot.active_timers.remove(&timer_ev.timer);
            let ::core::option::Option::Some(mut state) = existing else {
                return ::core::result::Result::Ok(());
            };
            let actions: ::std::vec::Vec<#commands_enum_ident> = match timer_ev.timer.as_str() {
                #(#timer_fire_arms,)*
                _ => return ::core::result::Result::Ok(()),
            };
            #commands_enum_ident::update_timers(&actions, &state, &mut snapshot.active_timers);
            let pending_commands = #commands_enum_ident::encode_pending(&actions)?;
            let mut snapshot = chromatofoor::integration::save_process_instance(
                store,
                &timer_ev.correlation_id,
                event.id,
                snapshot.active_timers,
                pending_commands,
                &state,
            )
            .await?;
            chromatofoor::integration::flush_process_commands::<Self, _, _>(
                store,
                &timer_ev.correlation_id,
                &mut snapshot,
                #commands_enum_ident::execute_pending,
            )
            .await
        }
    });

    let (dg_expr, idempotency_expr) =
        build_delivery_and_idempotency(delivery_guarantee, idempotency);

    let timer_bounds_for_apply: Vec<_> = timers
        .iter()
        .map(|t| {
            let timer_ident = &t.ident;
            quote! { #timer_ident: chromatofoor::integration::ProcessTimer<__State> }
        })
        .collect();
    let apply_where = if timer_bounds_for_apply.is_empty() {
        quote! {}
    } else {
        quote! { where #(#timer_bounds_for_apply),* }
    };
    let update_timers_body = if update_timer_arms.is_empty() {
        quote! {
            let _ = (actions, state, active_timers);
        }
    } else {
        quote! {
            let _ = state;
            for action in actions {
                match action {
                    #(#update_timer_arms)*
                    _ => {}
                }
            }
        }
    };

    quote! {
        #input
        #kind_impl

        const _: fn() = || {
            #(#domain_checks)*
        };

        #(#timer_defs)*

        #[derive(
            Clone,
            Debug,
            PartialEq,
            chromatofoor::architecturekit::Events,
        )]
        #[architecturekit(crate = "chromatofoor::architecturekit")]
        #vis enum #events_enum_ident {
            #(#event_variants,)*
        }

        #(#event_from_impls)*

        #[derive(
            Clone,
            Debug,
            PartialEq,
        )]
        #vis enum #commands_enum_ident {
            #(#command_variants,)*
        }

        #(#command_from_impls)*

        impl chromatofoor::integration::ProcessCommand for #commands_enum_ident {
            fn as_set_timer(&self) -> ::core::option::Option<&'static str> {
                match self {
                    #(#set_timer_arms)*
                    _ => ::core::option::Option::None,
                }
            }

            fn as_cancel_timer(&self) -> ::core::option::Option<&'static str> {
                match self {
                    #(#cancel_timer_arms)*
                    _ => ::core::option::Option::None,
                }
            }

            fn as_emitted_command(
                &self,
            ) -> ::core::option::Option<(&'static str, &'static str, &'static str, chromatofoor::serde_json::Value)> {
                match self {
                    #(#emitted_command_arms)*
                    #[allow(unreachable_patterns)]
                    _ => ::core::option::Option::None,
                }
            }
        }

        impl #commands_enum_ident {
            fn update_timers<__State>(
                actions: &[Self],
                state: &__State,
                active_timers: &mut ::std::collections::BTreeMap<
                    ::std::string::String,
                    chromatofoor::chrono::DateTime<chromatofoor::chrono::Utc>,
                >,
            ) #apply_where {
                #update_timers_body
            }

            fn encode_pending(
                actions: &[Self],
            ) -> ::core::result::Result<
                ::std::vec::Vec<chromatofoor::serde_json::Value>,
                chromatofoor::architecturekit::Error<
                    chromatofoor::architecturekit::DomainError,
                >,
            > {
                let mut out = ::std::vec::Vec::new();
                for action in actions {
                    match action {
                        #(#encode_pending_arms)*
                    }
                }
                ::core::result::Result::Ok(out)
            }

            async fn execute_pending(
                raw: chromatofoor::serde_json::Value,
                store: &chromatofoor::architecturekit::Store,
            ) -> ::core::result::Result<
                (),
                chromatofoor::architecturekit::Error<
                    chromatofoor::architecturekit::DomainError,
                >,
            > {
                let _ = (&raw, store);
                let kind = raw
                    .get("type")
                    .and_then(chromatofoor::serde_json::Value::as_str)
                    .unwrap_or_default();
                let _ = kind;
                #(#execute_pending_branches)*
                ::core::result::Result::Ok(())
            }
        }

        impl #impl_generics chromatofoor::integration::EventConsumer for #ident #ty_generics #where_clause {
            type Event = #events_enum_ident;
            type Error = chromatofoor::architecturekit::Error<
                chromatofoor::architecturekit::DomainError,
            >;

            const DELIVERY_GUARANTEE: chromatofoor::integration::DeliveryGuarantee = #dg_expr;
            const IDEMPOTENCY: ::core::option::Option<chromatofoor::integration::Idempotency> = #idempotency_expr;

            fn on_start(
                &self,
                store: &::std::sync::Arc<chromatofoor::architecturekit::Store>,
                token: &chromatofoor::architecturekit::CancellationToken,
            ) {
                let run = chromatofoor::integration::start_process_timers::<Self>(
                    ::std::sync::Arc::clone(store),
                    token.clone(),
                );
                chromatofoor::integration::supervise_process_timers::<Self>(run, token.clone());
            }

            async fn handle(
                &self,
                store: &chromatofoor::architecturekit::Store,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::core::result::Result<(), Self::Error> {
                match &event.data {
                    #(#event_arms,)*
                }
            }
        }

        impl #impl_generics chromatofoor::integration::ProcessManager for #ident #ty_generics #where_clause {
            type Command = #commands_enum_ident;

            fn is_ended(&self) -> bool {
                (#ends_when)(self)
            }

            fn step(
                state: &mut ::core::option::Option<Self>,
                ended: &mut bool,
                active_timers: &mut ::std::collections::BTreeSet<&'static str>,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::std::vec::Vec<Self::Command> {
                if *ended {
                    return ::std::vec::Vec::new();
                }
                let actions = match &event.data {
                    #(#step_event_arms,)*
                };
                for action in &actions {
                    if let ::core::option::Option::Some(t) = chromatofoor::integration::ProcessCommand::as_set_timer(action) {
                        active_timers.insert(t);
                    }
                    if let ::core::option::Option::Some(t) = chromatofoor::integration::ProcessCommand::as_cancel_timer(action) {
                        active_timers.remove(t);
                    }
                }
                if state.as_ref().is_some_and(Self::is_ended) {
                    *ended = true;
                    active_timers.clear();
                }
                actions
            }
        }
    }
    .into()
}
