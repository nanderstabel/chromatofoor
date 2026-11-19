use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Ident, Path, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
};

use crate::common::{
    IdSourceSpec, IdentifiedByAttr, ScopeAttr, authorized_impl_block, child_kind_impl,
    dedup_paths_with_variant_idents, extract_actors_attr, extract_doc_string, identified_impl,
};

/// Marks a type as an ESDM `bounded-context` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this bounded context belongs to.
pub fn bounded_context(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ScopeAttr { scope: scope_ty } = parse_macro_input!(attr as ScopeAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(
        ident,
        &scope_ty,
        "bounded-context",
        &name_str,
        &description_str,
    );

    quote! {
        #input
        #kind_impl
        impl chromatofoor::bounded_context::BoundedContext for #ident {}
        impl chromatofoor::bounded_context::event::EventStructScope for #ident {
            type BoundedContext = Self;
            type Domain = #scope_ty;
        }
        impl chromatofoor::integration::MappingEndpoint for #ident {}
    }
    .into()
}

/// Marks a struct as an ESDM `aggregate` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this aggregate belongs to.
/// - `identified_by = generated(uuid) | state(<field>) | static("<value>")`
///   *(optional, defaults to `generated(uuid)`)*: How instances of this aggregate are
///   identified.
pub fn aggregate(attr: TokenStream, item: TokenStream) -> TokenStream {
    let IdentifiedByAttr {
        scope: scope_ty,
        identified_by,
    } = parse_macro_input!(attr as IdentifiedByAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "aggregate", &name_str, &description_str);
    let is_struct = matches!(input.data, syn::Data::Struct(_));
    let id_impl = identified_impl(
        ident,
        &identified_by,
        IdSourceSpec::Generated(Ident::new("uuid", ident.span())),
        true,
        is_struct,
    );

    quote! {
        #input
        #kind_impl
        #id_impl
        impl chromatofoor::bounded_context::command::CommandScope for #ident {
            async fn execute<C: chromatofoor::bounded_context::Command<Self>>(
                store: &chromatofoor::architecturekit::Store,
                command: C,
            ) -> ::core::result::Result<
                ::std::vec::Vec<chromatofoor::architecturekit::eventsourcingdb::Event>,
                chromatofoor::bounded_context::CommandExecutionError<C::Error>,
            > {
                <Self as chromatofoor::bounded_context::Aggregate>::execute(store, command).await
            }
        }
        impl chromatofoor::bounded_context::Aggregate for #ident {}
        impl chromatofoor::bounded_context::event::EventStructScope for #ident {
            type BoundedContext = #scope_ty;
            type Domain = <#scope_ty as chromatofoor::common::Kind>::Scope;
        }
    }
    .into()
}

struct DcbAttr {
    scope: Path,
    identified_by: Vec<IdSourceSpec>,
    consults: Vec<Path>,
}

impl Parse for DcbAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut identified_by = Vec::new();
        let mut consults = Vec::new();
        let mut has_consults = false;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            if key == "scope" {
                scope = Some(input.parse()?);
            } else if key == "identified_by" {
                let mut specs = Vec::new();
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<IdSourceSpec, Token![,]>::parse_terminated(
                            &content,
                        )?;
                    specs.extend(parsed);
                } else {
                    specs.push(input.parse()?);
                }
                for spec in specs {
                    match &spec {
                        IdSourceSpec::State(field) => {
                            return Err(syn::Error::new_spanned(
                                field,
                                "`state(...)` is not allowed in `dynamic_consistency_boundary` `identified_by`; use `command_payload(...)`, `static(\"...\")`, or `generated(...)`",
                            ));
                        }
                        IdSourceSpec::Schema(field) => {
                            return Err(syn::Error::new_spanned(
                                field,
                                "`schema(...)` is only valid on `#[entity]`; use `command_payload(...)`, `static(\"...\")`, or `generated(...)`",
                            ));
                        }
                        _ => identified_by.push(spec),
                    }
                }
            } else if key == "consults" {
                let content;
                syn::bracketed!(content in input);
                let parsed =
                    syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                has_consults = true;
                consults.extend(parsed);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "expected `scope = <Type>`, `identified_by = ...`, or `consults = [<Event>, ...]`",
                ));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };
        if !has_consults {
            return Err(syn::Error::new(
                input.span(),
                "missing `consults = [<Event>, ...]`",
            ));
        }
        if consults.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "dynamic_consistency_boundary requires `consults = [<Event>, ...]` with at least one event",
            ));
        }

        Ok(Self {
            scope,
            identified_by,
            consults,
        })
    }
}

/// Marks a struct or enum as an ESDM `dynamic-consistency-boundary` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context whose events this boundary
///   reasons over.
/// - `identified_by = [command_payload(<field>), ...] | static("<value>") | generated(uuid)`
///   *(optional, defaults to `generated(uuid)`)*: How instances of this boundary are
///   identified.
/// - `consults = [<EventA>, <EventB>, ...]` *(required)*: The events within the bounded
///   context that are folded into or published by this boundary.
pub fn dynamic_consistency_boundary(attr: TokenStream, item: TokenStream) -> TokenStream {
    let DcbAttr {
        scope: scope_ty,
        identified_by,
        consults,
    } = parse_macro_input!(attr as DcbAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let vis = &input.vis;
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(
        ident,
        &scope_ty,
        "dynamic-consistency-boundary",
        &name_str,
        &description_str,
    );
    let is_struct = matches!(input.data, syn::Data::Struct(_));
    let id_impl = identified_impl(
        ident,
        &identified_by,
        IdSourceSpec::Generated(Ident::new("uuid", ident.span())),
        false,
        is_struct,
    );

    let event_enum_ident = Ident::new(&format!("{ident}Event"), ident.span());
    let mut variants = Vec::new();
    let mut from_impls = Vec::new();
    let mut evolve_arms = Vec::new();
    let mut event_on_arms = Vec::new();
    let mut bc_checks = Vec::new();

    for (var_ident, ev_path) in dedup_paths_with_variant_idents(&consults, "Event", ident.span()) {
        variants.push(quote! {
            #var_ident(#ev_path)
        });
        from_impls.push(quote! {
            impl ::core::convert::From<#ev_path> for #event_enum_ident {
                fn from(value: #ev_path) -> Self {
                    Self::#var_ident(value)
                }
            }
        });
        evolve_arms.push(quote! {
            #event_enum_ident::#var_ident(ev) => chromatofoor::bounded_context::Event::evolve(ev, self)
        });
        event_on_arms.push(quote! {
            Self::#var_ident(ev) => chromatofoor::architecturekit::EventOn::new(subject, ev)
        });
        let ev_span = ev_path.span();
        bc_checks.push(quote::quote_spanned! { ev_span =>
            chromatofoor::bounded_context::event::assert_event_in_bc::<#scope_ty, #ev_path>();
        });
    }

    let event_on_body = quote! {
        match self {
            #(#event_on_arms,)*
        }
    };

    let evolve_body = quote! {
        match event {
            #(#evolve_arms,)*
        }
    };

    quote! {
        #input
        #kind_impl
        #id_impl

        const _: fn() = || {
            #(#bc_checks)*
        };

        #[derive(
            Clone,
            Debug,
            PartialEq,
            chromatofoor::Events,
        )]
        #vis enum #event_enum_ident {
            #(#variants),*
        }

        #(#from_impls)*

        impl chromatofoor::bounded_context::dynamic_consistency_boundary::DcbEvents for #event_enum_ident {
            fn event_on(
                &self,
                subject: chromatofoor::architecturekit::Subject,
            ) -> chromatofoor::architecturekit::EventOn {
                #event_on_body
            }
        }

        impl chromatofoor::architecturekit::State for #ident {
            type Event = #event_enum_ident;

            fn evolve(self, event: &Self::Event) -> Self {
                #evolve_body
            }
        }

        impl chromatofoor::bounded_context::command::CommandScope for #ident {
            async fn execute<C: chromatofoor::bounded_context::Command<Self>>(
                store: &chromatofoor::architecturekit::Store,
                command: C,
            ) -> ::core::result::Result<
                ::std::vec::Vec<chromatofoor::architecturekit::eventsourcingdb::Event>,
                chromatofoor::bounded_context::CommandExecutionError<C::Error>,
            > {
                <Self as chromatofoor::bounded_context::DynamicConsistencyBoundary>::execute(
                    store, command,
                )
                .await
            }
        }
        impl chromatofoor::bounded_context::DynamicConsistencyBoundary for #ident {}
    }
    .into()
}

struct EventAttr {
    scope: Path,
}

impl Parse for EventAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "scope" {
                let _: Token![=] = input.parse()?;
                scope = Some(input.parse()?);
            } else {
                return Err(syn::Error::new_spanned(key, "expected `scope = <Type>`"));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };

        Ok(Self { scope })
    }
}

/// Marks an `enum` or `struct` as an ESDM `event`.
///
/// # Attributes
/// - `scope = <Aggregate>` *(when applied to an `enum`)*: Generates a struct for each
///   variant and implements `architecturekit::State` for the aggregate.
/// - `scope = <BoundedContext | Aggregate>` *(when applied to a `struct`)*: Marks the struct
///   as an ESDM event struct.
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let EventAttr { scope: scope_ty } = parse_macro_input!(attr as EventAttr);
    let scope_span = scope_ty.span();

    if let Ok(mut item_enum) = syn::parse::<syn::ItemEnum>(item.clone()) {
        if item_enum.variants.is_empty() {
            return syn::Error::new_spanned(
                &item_enum.ident,
                "an enum of events needs at least one variant",
            )
            .to_compile_error()
            .into();
        }
        let vis = &item_enum.vis;
        let enum_ident = &item_enum.ident;

        let scope_check = quote::quote_spanned! { scope_span =>
            chromatofoor::bounded_context::event::assert_aggregate_event_scope::<#scope_ty>();
        };

        let mut generated_structs = Vec::new();
        let mut from_impls = Vec::new();
        let mut evolve_arms = Vec::new();

        for variant in &mut item_enum.variants {
            let v_ident = variant.ident.clone();
            let v_attrs = variant.attrs.clone();
            let v_desc = extract_doc_string(&v_attrs);
            let name_str = v_ident.to_string().to_case(Case::Kebab);
            let v_extra_validate_crate_attr =
                if has_validate_derive(&v_attrs) && !has_validate_crate_attr(&v_attrs) {
                    quote! { #[validate(crate = "chromatofoor::validator")] }
                } else {
                    quote! {}
                };

            match &mut variant.fields {
                syn::Fields::Named(named) => {
                    let mut fields = named.named.clone();
                    for f in &mut fields {
                        f.vis = vis.clone();
                    }
                    let kind_impl =
                        child_kind_impl(&v_ident, &scope_ty, "event", &name_str, &v_desc);
                    generated_structs.push(quote! {
                        #(#v_attrs)*
                        #v_extra_validate_crate_attr
                        #[derive(
                            Clone,
                            Debug,
                            PartialEq,
                            chromatofoor::serde::Serialize,
                            chromatofoor::serde::Deserialize,
                            chromatofoor::Schema,
                        )]
                        #[serde(crate = "chromatofoor::serde")]
                        #vis struct #v_ident {
                            #fields
                        }

                        #kind_impl

                        impl chromatofoor::architecturekit::Event for #v_ident {
                            const EVENT_TYPE: &'static str = <Self as chromatofoor::common::Kind>::TOPIC;
                        }

                        impl chromatofoor::bounded_context::event::EventInBoundedContext<
                            <#scope_ty as chromatofoor::common::Kind>::Scope
                        > for #v_ident {}

                        impl chromatofoor::bounded_context::event::EventInDomain<
                            <<#scope_ty as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::Scope
                        > for #v_ident {}
                    });
                    from_impls.push(quote! {
                        impl ::core::convert::From<#v_ident> for #enum_ident {
                            fn from(value: #v_ident) -> Self {
                                Self::#v_ident(value)
                            }
                        }
                    });
                    variant.fields = syn::Fields::Unnamed(syn::parse_quote!((#v_ident)));
                }
                syn::Fields::Unit => {
                    let kind_impl =
                        child_kind_impl(&v_ident, &scope_ty, "event", &name_str, &v_desc);
                    generated_structs.push(quote! {
                        #(#v_attrs)*
                        #v_extra_validate_crate_attr
                        #[derive(
                            Clone,
                            Debug,
                            PartialEq,
                            chromatofoor::serde::Serialize,
                            chromatofoor::serde::Deserialize,
                            chromatofoor::Schema,
                        )]
                        #[serde(crate = "chromatofoor::serde")]
                        #vis struct #v_ident {}

                        #kind_impl

                        impl chromatofoor::architecturekit::Event for #v_ident {
                            const EVENT_TYPE: &'static str = <Self as chromatofoor::common::Kind>::TOPIC;
                        }

                        impl chromatofoor::bounded_context::event::EventInBoundedContext<
                            <#scope_ty as chromatofoor::common::Kind>::Scope
                        > for #v_ident {}

                        impl chromatofoor::bounded_context::event::EventInDomain<
                            <<#scope_ty as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::Scope
                        > for #v_ident {}
                    });
                    from_impls.push(quote! {
                        impl ::core::convert::From<#v_ident> for #enum_ident {
                            fn from(value: #v_ident) -> Self {
                                Self::#v_ident(value)
                            }
                        }
                    });
                    variant.fields = syn::Fields::Unnamed(syn::parse_quote!((#v_ident)));
                }
                syn::Fields::Unnamed(unnamed) => {
                    if unnamed.unnamed.len() != 1 {
                        return syn::Error::new_spanned(
                            variant,
                            "`#[chromatofoor::event]` tuple variants must have exactly one field `Variant(EventStruct)`",
                        )
                        .to_compile_error()
                        .into();
                    }
                    if let Some(first) = unnamed.unnamed.first() {
                        let inner_ty = &first.ty;
                        from_impls.push(quote! {
                            impl ::core::convert::From<#inner_ty> for #enum_ident {
                                fn from(value: #inner_ty) -> Self {
                                    Self::#v_ident(value)
                                }
                            }
                        });
                    }
                }
            }

            evolve_arms.push(quote! {
                #enum_ident::#v_ident(ev) => chromatofoor::bounded_context::Event::evolve(ev, self)
            });
        }

        item_enum.attrs.insert(
            0,
            syn::parse_quote! {
                #[derive(
                    Clone,
                    Debug,
                    PartialEq,
                    chromatofoor::Events,
                )]
            },
        );

        return quote! {
            const _: fn() = || {
                #scope_check
            };

            #(#generated_structs)*

            #item_enum

            #(#from_impls)*

            impl chromatofoor::architecturekit::State for #scope_ty {
                type Event = #enum_ident;

                fn evolve(self, event: &Self::Event) -> Self {
                    match event {
                        #(#evolve_arms,)*
                    }
                }
            }
        }
        .into();
    }

    let mut input = parse_macro_input!(item as DeriveInput);
    if let syn::Data::Struct(data_struct) = &mut input.data
        && matches!(data_struct.fields, syn::Fields::Unit)
    {
        data_struct.fields = syn::Fields::Named(syn::parse_quote!({}));
    }
    let scope_check = quote::quote_spanned! { scope_span =>
        chromatofoor::bounded_context::event::assert_freestanding_event_scope::<#scope_ty>();
    };
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "event", &name_str, &description_str);

    if has_validate_derive(&input.attrs) && !has_validate_crate_attr(&input.attrs) {
        input.attrs.push(syn::parse_quote! {
            #[validate(crate = "chromatofoor::validator")]
        });
    }
    input.attrs.insert(
        0,
        syn::parse_quote! {
            #[derive(
                Clone,
                Debug,
                PartialEq,
                chromatofoor::serde::Serialize,
                chromatofoor::serde::Deserialize,
                chromatofoor::Schema,
            )]
        },
    );
    input.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });

    quote! {
        const _: fn() = || {
            #scope_check
        };

        #input
        #kind_impl

        impl chromatofoor::architecturekit::Event for #ident {
            const EVENT_TYPE: &'static str = <Self as chromatofoor::common::Kind>::TOPIC;
        }

        impl chromatofoor::bounded_context::event::EventInBoundedContext<
            <#scope_ty as chromatofoor::bounded_context::event::EventStructScope>::BoundedContext
        > for #ident {}

        impl chromatofoor::bounded_context::event::EventInDomain<
            <#scope_ty as chromatofoor::bounded_context::event::EventStructScope>::Domain
        > for #ident {}
    }
    .into()
}

struct ActorAttr {
    scope: Path,
    actor_type: Ident,
    backed_by: Vec<Path>,
}

impl Parse for ActorAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut actor_type = None;
        let mut backed_by = Vec::new();
        let mut backed_by_span = None;

        while !input.is_empty() {
            if input.peek(Token![type]) {
                let _: Token![type] = input.parse()?;
                let _: Token![=] = input.parse()?;
                let ty: Ident = input.parse()?;
                if ty != "human" && ty != "system" {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "expected `type = human` or `type = system`",
                    ));
                }
                actor_type = Some(ty);
            } else {
                let key: Ident = input.parse()?;
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "backed_by" {
                    backed_by_span = Some(key.span());
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    backed_by.extend(parsed);
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope = <Type>`, `type = human | system`, or `backed_by = [<ExternalSystem>, ...]`",
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };
        let actor_type =
            actor_type.unwrap_or_else(|| Ident::new("human", proc_macro2::Span::call_site()));

        if actor_type == "human"
            && let Some(span) = backed_by_span
        {
            return Err(syn::Error::new(
                span,
                "`backed_by` is only allowed on `type = system` actors",
            ));
        }

        Ok(Self {
            scope,
            actor_type,
            backed_by,
        })
    }
}

/// Marks a type as an ESDM `domain-service` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this domain service belongs to.
pub fn domain_service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ScopeAttr { scope: scope_ty } = parse_macro_input!(attr as ScopeAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(
        ident,
        &scope_ty,
        "domain-service",
        &name_str,
        &description_str,
    );

    quote! {
        #input
        #kind_impl
        impl chromatofoor::bounded_context::DomainService for #ident {}
    }
    .into()
}

/// Marks a struct as an ESDM `actor` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this actor belongs to.
/// - `type = human | system` *(optional, defaults to `human`)*: Whether the actor is a
///   human role or an automated system trigger.
/// - `backed_by = [<ExternalSystem>, ...]` *(optional, only on `type = system`)*: External
///   systems that back this system actor.
pub fn actor(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ActorAttr {
        scope: scope_ty,
        actor_type,
        backed_by,
    } = parse_macro_input!(attr as ActorAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "actor", &name_str, &description_str);

    let actor_type_expr = if actor_type == "system" {
        quote! { chromatofoor::bounded_context::ActorType::System }
    } else {
        quote! { chromatofoor::bounded_context::ActorType::Human }
    };

    let mut backed_by_names = Vec::new();
    let mut domain_checks = Vec::new();
    for ext in &backed_by {
        backed_by_names.push(quote! {
            <#ext as chromatofoor::common::Kind>::NAME
        });
        let ext_span = ext.span();
        domain_checks.push(quote::quote_spanned! { ext_span =>
            chromatofoor::integration::external_system::assert_external_system_in_domain::<
                <#scope_ty as chromatofoor::common::Kind>::Scope,
                #ext,
            >();
            const _: () = assert!(
                !matches!(
                    <#ext as chromatofoor::integration::ExternalSystem>::DIRECTION,
                    chromatofoor::integration::ExternalDirection::Outbound,
                ),
                "`backed_by` on `#[actor]` requires an `inbound` or `bidirectional` external system",
            );
        });
    }

    let check_block = if domain_checks.is_empty() {
        quote! {}
    } else {
        quote! {
            const _: fn() = || {
                #(#domain_checks)*
            };
        }
    };

    quote! {
        #input
        #kind_impl
        #check_block
        impl chromatofoor::bounded_context::Actor for #ident {
            const ACTOR_TYPE: chromatofoor::bounded_context::ActorType = #actor_type_expr;
            const BACKED_BY: &'static [&'static str] = &[#(#backed_by_names),*];
        }
        impl chromatofoor::bounded_context::actor::ActorInBoundedContext<#scope_ty> for #ident {}
    }
    .into()
}

struct CommandAttr {
    scope: Path,
    is_create: bool,
    actors: Vec<Path>,
    error: Option<Path>,
    validate: bool,
}

impl Parse for CommandAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut is_create = false;
        let mut actors = Vec::new();
        let mut error = None;
        let mut validate = false;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "creates" {
                is_create = true;
            } else if key == "validate" {
                validate = true;
            } else {
                let _: Token![=] = input.parse()?;
                if key == "scope" {
                    scope = Some(input.parse()?);
                } else if key == "error" {
                    error = Some(input.parse()?);
                } else if key == "lifecycle" {
                    let val: Ident = input.parse()?;
                    if val == "create" {
                        is_create = true;
                    } else if val != "mutate" {
                        return Err(syn::Error::new_spanned(
                            val,
                            "expected `lifecycle = create` or `lifecycle = mutate`",
                        ));
                    }
                } else if key == "actors" {
                    let content;
                    syn::bracketed!(content in input);
                    let parsed =
                        syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                    actors.extend(parsed);
                } else {
                    return Err(syn::Error::new_spanned(
                        key,
                        "expected `scope = <Type>`, `error = <ErrorType>`, `validate`, `lifecycle = create`, `creates`, or `actors = [...]`",
                    ));
                }
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
            is_create,
            actors,
            error,
            validate,
        })
    }
}

fn extract_lifecycle_create(attrs: &mut Vec<syn::Attribute>) -> syn::Result<bool> {
    let mut is_create = false;
    let mut err: Option<syn::Error> = None;
    let mut record_err = |e: syn::Error| {
        if let Some(existing) = &mut err {
            existing.combine(e);
        } else {
            err = Some(e);
        }
    };
    attrs.retain(|attr| {
        if attr.path().is_ident("creates") {
            if let Err(e) = attr.meta.require_path_only() {
                record_err(e);
            } else {
                is_create = true;
            }
            return false;
        }
        if attr.path().is_ident("lifecycle") {
            match attr.parse_args::<Ident>() {
                Ok(ident) if ident == "create" => {
                    is_create = true;
                }
                Ok(ident) => {
                    record_err(syn::Error::new_spanned(
                        ident,
                        "expected `#[lifecycle(create)]`",
                    ));
                }
                Err(e) => record_err(e),
            }
            return false;
        }
        true
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(is_create)
}

fn command_impl_block(
    ident: &Ident,
    scope_ty: &Path,
    fields: &[(Ident, syn::Type)],
    is_create: bool,
    validate: bool,
) -> proc_macro2::TokenStream {
    let field_name_strs: Vec<String> = fields.iter().map(|(id, _)| id.to_string()).collect();
    let field_meta = fields
        .iter()
        .zip(field_name_strs.iter())
        .map(|((_, f_ty), f_str)| {
            quote! {
                (
                    #f_str,
                    {
                        #[allow(unused_imports)]
                        use chromatofoor::common::NonValidIdField as _;
                        chromatofoor::common::IdFieldProbe::<#f_ty>::IS_VALID
                    }
                )
            }
        });
    let field_pairs = fields
        .iter()
        .zip(field_name_strs.iter())
        .map(|((f_ident, _), f_str)| {
            quote! {
                (
                    #f_str,
                    {
                        #[allow(unused_imports)]
                        use chromatofoor::common::{FieldAsIdStr as _, NonIdField as _};
                        (&self.#f_ident).as_id_str()
                    }
                )
            }
        });

    let preconditions_body = if is_create {
        quote! {
            ::std::vec![chromatofoor::architecturekit::Precondition::on_state_read()]
        }
    } else {
        quote! {
            if <#scope_ty as chromatofoor::common::Identified>::HAS_LIFECYCLE {
                ::std::vec![
                    chromatofoor::architecturekit::Precondition::on_populated_subject(
                        chromatofoor::architecturekit::Command::subject(self),
                    ),
                    chromatofoor::architecturekit::Precondition::on_state_read(),
                ]
            } else {
                ::std::vec![chromatofoor::architecturekit::Precondition::on_state_read()]
            }
        }
    };

    let validate_impl = if validate {
        quote! {
            impl<__E: ::core::convert::From<chromatofoor::validator::ValidationErrors>>
                chromatofoor::bounded_context::command::ValidatePayload<__E> for #ident
            {
                fn validate_payload(&self) -> ::core::result::Result<(), __E> {
                    chromatofoor::validator::Validate::validate(self)
                        .map_err(::core::convert::Into::into)
                }
            }
        }
    } else {
        quote! {
            impl<__E> chromatofoor::bounded_context::command::ValidatePayload<__E> for #ident {}
        }
    };

    quote! {
        const _: () = {
            assert!(
                chromatofoor::common::validate_command_id_fields(
                    <#scope_ty as chromatofoor::common::Identified>::IDENTIFIED_BY,
                    &[#(#field_meta),*],
                    #is_create,
                ),
                "command is missing an `identified_by` field required by its scope, or that field does not have a valid ID type (`Identifier`, `SubjectSegment`, or `Uuid`)",
            );
        };

        impl chromatofoor::architecturekit::Command for #ident {
            fn subject(&self) -> chromatofoor::architecturekit::Subject {
                chromatofoor::common::resolve_subject::<#scope_ty>(&[
                    #(#field_pairs),*
                ])
            }

            fn preconditions(&self) -> ::std::vec::Vec<chromatofoor::architecturekit::Precondition> {
                #preconditions_body
            }
        }

        #validate_impl

        impl chromatofoor::bounded_context::command::CommandInDomain<
            <<#scope_ty as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::Scope
        > for #ident {}
    }
}

fn has_validate_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "Validate")
            {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn has_validate_crate_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("validate") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// Marks an `enum` or `struct` as an ESDM `command` scoped to an `Aggregate` or
/// `DynamicConsistencyBoundary`.
///
/// # Attributes
/// - `scope = <Aggregate | DynamicConsistencyBoundary>` *(required)*: The consistency unit
///   these commands target.
/// - `creates` or `lifecycle = create | mutate` *(optional)*: Marks a command struct (or
///   all variants of a command enum) as an aggregate creation command. Individual enum
///   variants can also be annotated with `#[creates]` or `#[lifecycle(create)]`.
/// - `actors = [<Actor>, ...]` *(optional)*: Default list of actors permitted to issue this
///   command (individual enum variants or structs can also add `#[actors(...)]`).
/// - `error = <ErrorType>` *(optional, enum only)*: Error type for the generated
///   `Command<Scope>` implementation on the enum (defaults to the first variant's `Error`).
/// - `validate` *(optional)*: Runs `validator::Validate::validate` before `Command::decide`.
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let CommandAttr {
        scope: scope_ty,
        is_create: attr_is_create,
        actors: attr_actors,
        error: attr_error,
        validate: attr_validate,
    } = parse_macro_input!(attr as CommandAttr);
    let scope_span = scope_ty.span();
    let scope_check = quote::quote_spanned! { scope_span =>
        chromatofoor::bounded_context::command::assert_command_scope::<#scope_ty>();
    };
    let bc_ty = quote! { <#scope_ty as chromatofoor::common::Kind>::Scope };

    let Ok(mut item_enum) = syn::parse::<syn::ItemEnum>(item.clone()) else {
        let mut input = parse_macro_input!(item as DeriveInput);
        if let Some(err_ty) = attr_error {
            return syn::Error::new_spanned(
                err_ty,
                "`error = ...` is only supported on command `enum`s; for a command `struct`, specify `type Error` in `impl Command<Scope>`",
            )
            .to_compile_error()
            .into();
        }
        let attr_lifecycle_create = match extract_lifecycle_create(&mut input.attrs) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error().into(),
        };
        let is_create = attr_is_create || attr_lifecycle_create;
        let mut s_actors = attr_actors.clone();
        match extract_actors_attr(&mut input.attrs) {
            Ok(parsed) => s_actors.extend(parsed),
            Err(e) => return e.to_compile_error().into(),
        }
        let mut field_defs = Vec::new();
        if let syn::Data::Struct(data_struct) = &mut input.data {
            if matches!(data_struct.fields, syn::Fields::Unit) {
                data_struct.fields = syn::Fields::Named(syn::parse_quote!({}));
            }
            if let syn::Fields::Named(named) = &data_struct.fields {
                for f in &named.named {
                    if let Some(id) = &f.ident {
                        field_defs.push((id.clone(), f.ty.clone()));
                    }
                }
            }
        } else {
            return syn::Error::new_spanned(
                input.ident,
                "`#[command]` must be applied to an `enum` or `struct`",
            )
            .to_compile_error()
            .into();
        }
        let ident = &input.ident;
        let name_str = ident.to_string().to_case(Case::Kebab);
        let desc = extract_doc_string(&input.attrs);
        let kind_impl = child_kind_impl(ident, &scope_ty, "command", &name_str, &desc);
        let auth_impl = authorized_impl_block(ident, &bc_ty, &s_actors);
        let cmd_impl = command_impl_block(ident, &scope_ty, &field_defs, is_create, attr_validate);

        let had_validate_derive = has_validate_derive(&input.attrs);
        let needs_validate_derive = attr_validate && !had_validate_derive;
        let needs_validate_crate_attr =
            (attr_validate || had_validate_derive) && !has_validate_crate_attr(&input.attrs);
        if needs_validate_derive {
            input.attrs.insert(
                0,
                syn::parse_quote! {
                    #[derive(
                        Clone,
                        Debug,
                        PartialEq,
                        chromatofoor::serde::Serialize,
                        chromatofoor::serde::Deserialize,
                        chromatofoor::Schema,
                        chromatofoor::validator::Validate,
                    )]
                },
            );
        } else {
            input.attrs.insert(
                0,
                syn::parse_quote! {
                    #[derive(
                        Clone,
                        Debug,
                        PartialEq,
                        chromatofoor::serde::Serialize,
                        chromatofoor::serde::Deserialize,
                        chromatofoor::Schema,
                    )]
                },
            );
        }
        if needs_validate_crate_attr {
            input.attrs.push(syn::parse_quote! {
                #[validate(crate = "chromatofoor::validator")]
            });
        }
        input.attrs.push(syn::parse_quote! {
            #[serde(crate = "chromatofoor::serde")]
        });
        let validate_trait_import = if attr_validate || had_validate_derive {
            quote! {
                #[allow(unused_imports)]
                use chromatofoor::validator::Validate as _;
            }
        } else {
            quote! {}
        };

        return quote! {
            #validate_trait_import
            const _: fn() = || {
                #scope_check
            };

            #input
            #kind_impl
            #cmd_impl
            #auth_impl
        }
        .into();
    };

    let vis = &item_enum.vis;
    let enum_ident = &item_enum.ident;

    let mut generated_structs = Vec::new();
    let mut from_impls = Vec::new();
    let mut subject_arms = Vec::new();
    let mut precondition_arms = Vec::new();
    let mut decide_arms = Vec::new();
    let mut validate_arms = Vec::new();
    let mut matches_arms = Vec::new();
    let mut auth_arms = Vec::new();
    let mut auth_actor_arms = Vec::new();
    let mut tuple_actor_checks = Vec::new();
    let mut tuple_allowed_actor_checks = Vec::new();
    let mut all_enum_actors = attr_actors.clone();
    let mut first_variant_ty: Option<syn::Type> = None;
    let mut any_validate_derive = attr_validate;

    for variant in &mut item_enum.variants {
        let v_ident = variant.ident.clone();
        if first_variant_ty.is_none() {
            if let syn::Fields::Unnamed(unnamed) = &variant.fields
                && let Some(first) = unnamed.unnamed.first()
            {
                first_variant_ty = Some(first.ty.clone());
            }
            if first_variant_ty.is_none() {
                first_variant_ty = Some(syn::parse_quote!(#v_ident));
            }
        }
        let v_lifecycle_create = match extract_lifecycle_create(&mut variant.attrs) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error().into(),
        };
        let v_is_create = attr_is_create || v_lifecycle_create;
        let v_own_actors = match extract_actors_attr(&mut variant.attrs) {
            Ok(parsed) => parsed,
            Err(e) => return e.to_compile_error().into(),
        };
        let mut v_actors = attr_actors.clone();
        v_actors.extend(v_own_actors);
        for a in &v_actors {
            let a_str = quote!(#a).to_string();
            if !all_enum_actors
                .iter()
                .any(|existing| quote!(#existing).to_string() == a_str)
            {
                all_enum_actors.push(a.clone());
            }
        }
        let v_attrs = variant.attrs.clone();
        let v_desc = extract_doc_string(&v_attrs);
        let name_str = v_ident.to_string().to_case(Case::Kebab);
        let kind_impl = child_kind_impl(&v_ident, &scope_ty, "command", &name_str, &v_desc);
        let auth_impl = authorized_impl_block(&v_ident, &bc_ty, &v_actors);
        let mut tuple_inner_ty: Option<syn::Type> = None;
        let v_had_validate_derive = has_validate_derive(&v_attrs);
        if v_had_validate_derive {
            any_validate_derive = true;
        }
        let v_extra_validate_derive = if attr_validate && !v_had_validate_derive {
            quote! { chromatofoor::validator::Validate, }
        } else {
            quote! {}
        };
        let v_extra_validate_crate_attr =
            if (attr_validate || v_had_validate_derive) && !has_validate_crate_attr(&v_attrs) {
                quote! { #[validate(crate = "chromatofoor::validator")] }
            } else {
                quote! {}
            };

        match &mut variant.fields {
            syn::Fields::Named(named) => {
                let mut fields = named.named.clone();
                let mut field_defs = Vec::new();
                for f in &mut fields {
                    f.vis = vis.clone();
                    if let Some(id) = &f.ident {
                        field_defs.push((id.clone(), f.ty.clone()));
                    }
                }
                let cmd_impl = command_impl_block(
                    &v_ident,
                    &scope_ty,
                    &field_defs,
                    v_is_create,
                    attr_validate,
                );
                generated_structs.push(quote! {
                    #[derive(
                        Clone,
                        Debug,
                        PartialEq,
                        chromatofoor::serde::Serialize,
                        chromatofoor::serde::Deserialize,
                        chromatofoor::Schema,
                        #v_extra_validate_derive
                    )]
                    #(#v_attrs)*
                    #v_extra_validate_crate_attr
                    #[serde(crate = "chromatofoor::serde")]
                    #vis struct #v_ident {
                        #fields
                    }

                    #kind_impl
                    #cmd_impl
                    #auth_impl
                });
                from_impls.push(quote! {
                    impl ::core::convert::From<#v_ident> for #enum_ident {
                        fn from(value: #v_ident) -> Self {
                            Self::#v_ident(value)
                        }
                    }
                });
                variant.fields = syn::Fields::Unnamed(syn::parse_quote!((#v_ident)));
            }
            syn::Fields::Unit => {
                let cmd_impl =
                    command_impl_block(&v_ident, &scope_ty, &[], v_is_create, attr_validate);
                generated_structs.push(quote! {
                    #[derive(
                        Clone,
                        Debug,
                        PartialEq,
                        chromatofoor::serde::Serialize,
                        chromatofoor::serde::Deserialize,
                        chromatofoor::Schema,
                        #v_extra_validate_derive
                    )]
                    #(#v_attrs)*
                    #v_extra_validate_crate_attr
                    #[serde(crate = "chromatofoor::serde")]
                    #vis struct #v_ident {}

                    #kind_impl
                    #cmd_impl
                    #auth_impl
                });
                from_impls.push(quote! {
                    impl ::core::convert::From<#v_ident> for #enum_ident {
                        fn from(value: #v_ident) -> Self {
                            Self::#v_ident(value)
                        }
                    }
                });
                variant.fields = syn::Fields::Unnamed(syn::parse_quote!((#v_ident)));
            }
            syn::Fields::Unnamed(unnamed) => {
                if v_lifecycle_create {
                    return syn::Error::new_spanned(
                        &variant.ident,
                        "`#[creates]` / `#[lifecycle(create)]` must be placed on the command struct definition, not on a tuple variant",
                    )
                    .to_compile_error()
                    .into();
                }
                for actor_path in &v_actors {
                    let actor_span = actor_path.span();
                    tuple_actor_checks.push(quote::quote_spanned! { actor_span =>
                        chromatofoor::bounded_context::actor::assert_actor_in_bc::<#bc_ty, #actor_path>();
                    });
                }
                if let Some(first) = unnamed.unnamed.first() {
                    let inner_ty = &first.ty;
                    tuple_inner_ty = Some(inner_ty.clone());
                    tuple_allowed_actor_checks.push(quote! {
                        <#inner_ty as chromatofoor::bounded_context::Authorized>::is_allowed_actor(actor_name)
                    });
                    from_impls.push(quote! {
                        impl ::core::convert::From<#inner_ty> for #enum_ident {
                            fn from(value: #inner_ty) -> Self {
                                Self::#v_ident(value)
                            }
                        }
                    });
                }
            }
        }

        subject_arms.push(quote! {
            #enum_ident::#v_ident(ref cmd) => chromatofoor::architecturekit::Command::subject(cmd)
        });
        precondition_arms.push(quote! {
            #enum_ident::#v_ident(ref cmd) => chromatofoor::architecturekit::Command::preconditions(cmd)
        });
        if attr_validate {
            validate_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::validator::Validate::validate(cmd).map_err(::core::convert::Into::into)
            });
            decide_arms.push(quote! {
                #enum_ident::#v_ident(cmd) => {
                    chromatofoor::validator::Validate::validate(&cmd)?;
                    chromatofoor::bounded_context::Command::decide(cmd, state).await
                }
            });
        } else {
            validate_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::command::ValidatePayload::validate_payload(cmd)
            });
            decide_arms.push(quote! {
                #enum_ident::#v_ident(cmd) => {
                    chromatofoor::bounded_context::command::ValidatePayload::<Self::Error>::validate_payload(&cmd)?;
                    chromatofoor::bounded_context::Command::decide(cmd, state).await
                }
            });
        }
        matches_arms.push(quote! {
            #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::Command::matches(cmd, event)
        });
        if v_actors.is_empty() {
            auth_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::Authorized::is_authorized_for(cmd, actor_name)
            });
            auth_actor_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::Authorized::is_authorized_for_actor::<__A>(cmd)
            });
        } else if let Some(inner_ty) = tuple_inner_ty {
            auth_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => {
                    [#(<#v_actors as chromatofoor::common::Kind>::NAME),*].contains(&actor_name)
                        && (<#inner_ty as chromatofoor::bounded_context::Authorized>::ALLOWED_ACTORS.is_empty()
                            || chromatofoor::bounded_context::Authorized::is_authorized_for(cmd, actor_name))
                }
            });
            auth_actor_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => {
                    <Self as chromatofoor::bounded_context::Authorized>::BOUNDED_CONTEXT
                        == <<__A as chromatofoor::common::Kind>::Scope as chromatofoor::common::Kind>::NAME
                        && [
                            #((
                                <#v_actors as chromatofoor::common::Kind>::NAME,
                                <#v_actors as chromatofoor::bounded_context::Actor>::ACTOR_TYPE,
                            )),*
                        ]
                        .contains(&(
                            <__A as chromatofoor::common::Kind>::NAME,
                            <__A as chromatofoor::bounded_context::Actor>::ACTOR_TYPE,
                        ))
                        && (<#inner_ty as chromatofoor::bounded_context::Authorized>::ALLOWED_ACTORS.is_empty()
                            || chromatofoor::bounded_context::Authorized::is_authorized_for_actor::<__A>(cmd))
                }
            });
        } else {
            auth_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::Authorized::is_authorized_for(cmd, actor_name)
            });
            auth_actor_arms.push(quote! {
                #enum_ident::#v_ident(ref cmd) => chromatofoor::bounded_context::Authorized::is_authorized_for_actor::<__A>(cmd)
            });
        }
    }

    let enum_name_str = enum_ident.to_string().to_case(Case::Kebab);
    let enum_desc = extract_doc_string(&item_enum.attrs);
    let enum_kind_impl =
        child_kind_impl(enum_ident, &scope_ty, "command", &enum_name_str, &enum_desc);
    let mut enum_actor_checks = Vec::new();
    let mut enum_actor_entries = Vec::new();
    for actor_path in &all_enum_actors {
        let actor_span = actor_path.span();
        enum_actor_checks.push(quote::quote_spanned! { actor_span =>
            chromatofoor::bounded_context::actor::assert_actor_in_bc::<#bc_ty, #actor_path>();
        });
        enum_actor_entries.push(quote! {
            (
                <#actor_path as chromatofoor::common::Kind>::NAME,
                <#actor_path as chromatofoor::bounded_context::Actor>::ACTOR_TYPE,
            )
        });
    }
    let enum_auth_impl = quote! {
        const _: fn() = || {
            #(#enum_actor_checks)*
            #(#tuple_actor_checks)*
        };

        impl chromatofoor::bounded_context::Authorized for #enum_ident {
            const BOUNDED_CONTEXT: &'static str =
                <#bc_ty as chromatofoor::common::Kind>::NAME;
            const ALLOWED_ACTORS: &'static [(
                &'static str,
                chromatofoor::bounded_context::ActorType,
            )] = &[#(#enum_actor_entries),*];

            #[allow(unused_variables)]
            fn is_allowed_actor(actor_name: &str) -> bool {
                Self::ALLOWED_ACTORS
                    .iter()
                    .any(|(name, _)| *name == actor_name)
                    #(|| #tuple_allowed_actor_checks)*
            }

            #[allow(unused_variables)]
            fn is_authorized_for(&self, actor_name: &str) -> bool {
                match *self {
                    #(#auth_arms,)*
                }
            }

            fn is_authorized_for_actor<__A: chromatofoor::bounded_context::Actor>(&self) -> bool {
                match *self {
                    #(#auth_actor_arms,)*
                }
            }
        }
    };
    let resolved_error_ty: Option<proc_macro2::TokenStream> = if let Some(err_path) = attr_error {
        Some(quote! { #err_path })
    } else {
        first_variant_ty
            .map(|first_ty| quote! { <#first_ty as chromatofoor::bounded_context::Command<#scope_ty>>::Error })
    };
    let enum_validate_impl = if let Some(ref error_ty) = resolved_error_ty {
        quote! {
            impl chromatofoor::bounded_context::command::ValidatePayload<#error_ty> for #enum_ident {
                fn validate_payload(&self) -> ::core::result::Result<(), #error_ty> {
                    match *self {
                        #(#validate_arms,)*
                    }
                }
            }
        }
    } else {
        quote! {
            impl<__E> chromatofoor::bounded_context::command::ValidatePayload<__E> for #enum_ident {}
        }
    };
    let enum_command_impl = if let Some(error_ty) = resolved_error_ty {
        quote! {
            impl chromatofoor::bounded_context::Command<#scope_ty> for #enum_ident {
                type Error = #error_ty;

                #[allow(unused_variables)]
                async fn decide(
                    self,
                    state: &#scope_ty,
                ) -> ::core::result::Result<
                    ::std::vec::Vec<<#scope_ty as chromatofoor::architecturekit::State>::Event>,
                    Self::Error,
                > {
                    match self {
                        #(#decide_arms,)*
                    }
                }

                #[allow(unused_variables)]
                fn matches(
                    &self,
                    event: &chromatofoor::architecturekit::Envelope<
                        <#scope_ty as chromatofoor::architecturekit::State>::Event,
                    >,
                ) -> bool {
                    match *self {
                        #(#matches_arms,)*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    item_enum.attrs.push(syn::parse_quote! {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            chromatofoor::serde::Serialize,
            chromatofoor::serde::Deserialize,
        )]
    });
    item_enum.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });
    let validate_trait_import = if any_validate_derive {
        quote! {
            #[allow(unused_imports)]
            use chromatofoor::validator::Validate as _;
        }
    } else {
        quote! {}
    };

    quote! {
        #validate_trait_import
        const _: fn() = || {
            #scope_check
        };

        #(#generated_structs)*

        #item_enum
        #(#from_impls)*
        #enum_kind_impl
        #enum_auth_impl
        #enum_validate_impl

        impl chromatofoor::architecturekit::Schema for #enum_ident {
            fn schema() -> chromatofoor::serde_json::Value {
                chromatofoor::serde_json::json!({ "type": "object" })
            }
        }

        impl chromatofoor::architecturekit::Command for #enum_ident {
            fn subject(&self) -> chromatofoor::architecturekit::Subject {
                match *self {
                    #(#subject_arms,)*
                }
            }

            fn preconditions(&self) -> ::std::vec::Vec<chromatofoor::architecturekit::Precondition> {
                match *self {
                    #(#precondition_arms,)*
                }
            }
        }

        #enum_command_impl
    }
    .into()
}

struct QueryAttr {
    scope: Path,
    actors: Vec<Path>,
}

impl Parse for QueryAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut actors = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            if key == "scope" {
                scope = Some(input.parse()?);
            } else if key == "actors" {
                let content;
                syn::bracketed!(content in input);
                let parsed =
                    syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                actors.extend(parsed);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "expected `scope = <Type>` or `actors = [<Type>, ...]`",
                ));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };

        Ok(Self { scope, actors })
    }
}

/// Marks a struct as an ESDM `query` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this query belongs to.
/// - `actors = [<Actor>, ...]` *(optional)*: The actors permitted to run this query.
pub fn query(attr: TokenStream, item: TokenStream) -> TokenStream {
    let QueryAttr {
        scope: scope_ty,
        mut actors,
    } = parse_macro_input!(attr as QueryAttr);
    let mut input = parse_macro_input!(item as DeriveInput);
    match extract_actors_attr(&mut input.attrs) {
        Ok(parsed) => actors.extend(parsed),
        Err(e) => return e.to_compile_error().into(),
    }
    if let syn::Data::Struct(data_struct) = &mut input.data
        && matches!(data_struct.fields, syn::Fields::Unit)
    {
        data_struct.fields = syn::Fields::Named(syn::parse_quote!({}));
    }
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "query", &name_str, &description_str);
    let bc_ty = quote! { #scope_ty };
    let auth_impl = authorized_impl_block(ident, &bc_ty, &actors);

    if has_validate_derive(&input.attrs) && !has_validate_crate_attr(&input.attrs) {
        input.attrs.push(syn::parse_quote! {
            #[validate(crate = "chromatofoor::validator")]
        });
    }
    input.attrs.insert(
        0,
        syn::parse_quote! {
            #[derive(
                Clone,
                Debug,
                PartialEq,
                chromatofoor::serde::Serialize,
                chromatofoor::serde::Deserialize,
                chromatofoor::Schema,
            )]
        },
    );
    input.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });

    quote! {
        #input
        #kind_impl
        #auth_impl
    }
    .into()
}

/// Marks a struct as an ESDM `entity` scoped to a `BoundedContext`.
///
/// Automatically derives `Clone`, `Debug`, `PartialEq`, `serde::Serialize`,
/// `serde::Deserialize`, and `architecturekit::Schema`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this entity belongs to.
/// - `identified_by = schema(<field>) | static("<value>")` *(optional, defaults to
///   `schema(id)`)*: How instances of this entity are identified.
pub fn entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let IdentifiedByAttr {
        scope: scope_ty,
        identified_by,
    } = parse_macro_input!(attr as IdentifiedByAttr);
    for spec in &identified_by {
        match spec {
            IdSourceSpec::Schema(_) | IdSourceSpec::Static(_) => {}
            IdSourceSpec::State(f)
            | IdSourceSpec::CommandPayload(f)
            | IdSourceSpec::Generated(f) => {
                return syn::Error::new_spanned(
                    f,
                    "`#[entity]` `identified_by` only accepts `schema(<field>)` or `static(\"...\")`",
                )
                .to_compile_error()
                .into();
            }
        }
    }
    let mut input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "entity", &name_str, &description_str);
    let is_struct = matches!(input.data, syn::Data::Struct(_));
    let id_impl = identified_impl(
        ident,
        &identified_by,
        IdSourceSpec::Schema(Ident::new("id", ident.span())),
        false,
        is_struct,
    );

    if has_validate_derive(&input.attrs) && !has_validate_crate_attr(&input.attrs) {
        input.attrs.push(syn::parse_quote! {
            #[validate(crate = "chromatofoor::validator")]
        });
    }
    input.attrs.insert(
        0,
        syn::parse_quote! {
            #[derive(
                Clone,
                Debug,
                PartialEq,
                chromatofoor::serde::Serialize,
                chromatofoor::serde::Deserialize,
                chromatofoor::Schema,
            )]
        },
    );
    input.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });

    quote! {
        #input
        #kind_impl
        #id_impl
        impl chromatofoor::bounded_context::Entity for #ident {}
    }
    .into()
}

/// Marks a struct or unit-variant enum as an ESDM `value-object` scoped to a `BoundedContext`.
///
/// Automatically derives `Clone`, `Debug`, `PartialEq`, `serde::Serialize`,
/// `serde::Deserialize`, and `chromatofoor::Schema` (implementing `architecturekit::Schema`).
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this value object belongs to.
pub fn value_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ScopeAttr { scope: scope_ty } = parse_macro_input!(attr as ScopeAttr);
    let mut input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(
        ident,
        &scope_ty,
        "value-object",
        &name_str,
        &description_str,
    );

    if has_validate_derive(&input.attrs) && !has_validate_crate_attr(&input.attrs) {
        input.attrs.push(syn::parse_quote! {
            #[validate(crate = "chromatofoor::validator")]
        });
    }
    input.attrs.insert(
        0,
        syn::parse_quote! {
            #[derive(
                Clone,
                Debug,
                PartialEq,
                chromatofoor::serde::Serialize,
                chromatofoor::serde::Deserialize,
                chromatofoor::Schema,
            )]
        },
    );
    input.attrs.push(syn::parse_quote! {
        #[serde(crate = "chromatofoor::serde")]
    });

    quote! {
        #input
        #kind_impl
        impl chromatofoor::bounded_context::ValueObject for #ident {}
    }
    .into()
}

struct ReadModelAttr {
    scope: Path,
    events: Vec<Path>,
}

impl Parse for ReadModelAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut scope = None;
        let mut events = Vec::new();
        let mut has_events = false;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            if key == "scope" {
                scope = Some(input.parse()?);
            } else if key == "events" {
                let content;
                syn::bracketed!(content in input);
                let parsed =
                    syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
                has_events = true;
                events.extend(parsed);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "expected `scope = <Type>` or `events = [<Type>, ...]`",
                ));
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(scope) = scope else {
            return Err(syn::Error::new(input.span(), "missing `scope = <Type>`"));
        };
        if !has_events {
            return Err(syn::Error::new(
                input.span(),
                "missing `events = [<Event>, ...]`",
            ));
        }
        if events.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "read_model requires `events = [<Event>, ...]` with at least one event",
            ));
        }

        Ok(Self { scope, events })
    }
}

pub(crate) fn build_events_enum_and_dispatch(
    vis: &syn::Visibility,
    events_enum_ident: &Ident,
    events: &[Path],
    call_expr: impl Fn(&Path) -> proc_macro2::TokenStream,
) -> (proc_macro2::TokenStream, Vec<proc_macro2::TokenStream>) {
    let mut variants = Vec::new();
    let mut from_impls = Vec::new();
    let mut match_arms = Vec::new();

    for (var_ident, ev_path) in
        dedup_paths_with_variant_idents(events, "Event", events_enum_ident.span())
    {
        variants.push(quote! {
            #var_ident(#ev_path)
        });
        from_impls.push(quote! {
            impl ::core::convert::From<#ev_path> for #events_enum_ident {
                fn from(value: #ev_path) -> Self {
                    Self::#var_ident(value)
                }
            }
        });
        let call = call_expr(&ev_path);
        match_arms.push(quote! {
            #events_enum_ident::#var_ident(_) => {
                let mapped = event.map(|data| match data {
                    #events_enum_ident::#var_ident(inner) => inner,
                    #[allow(unreachable_patterns, clippy::unreachable)]
                    _ => unreachable!(),
                });
                #call
            }
        });
    }

    let enum_def = quote! {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            chromatofoor::Events,
        )]
        #vis enum #events_enum_ident {
            #(#variants,)*
        }

        #(#from_impls)*
    };

    (enum_def, match_arms)
}

/// Marks a struct as an ESDM `read-model` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context that owns this read model.
/// - `events = [<EventA>, <EventB>, ...]` *(required)*: The events projected into this read
///   model via `Project<E>`.
pub fn read_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ReadModelAttr {
        scope: scope_ty,
        events,
    } = parse_macro_input!(attr as ReadModelAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let vis = &input.vis;
    let ident = &input.ident;
    let name_str = ident.to_string().to_case(Case::Kebab);
    let description_str = extract_doc_string(&input.attrs);
    let kind_impl = child_kind_impl(ident, &scope_ty, "read-model", &name_str, &description_str);

    let events_enum_ident = Ident::new(&format!("{ident}Events"), ident.span());
    let (events_enum_def, match_arms) = build_events_enum_and_dispatch(
        vis,
        &events_enum_ident,
        &events,
        |ev_path| {
            quote! {
                <Self as chromatofoor::bounded_context::Project<#ev_path>>::project(self, mapped).await
            }
        },
    );

    let mut seen_calls = Vec::new();
    if let syn::Data::Struct(data_struct) = &input.data {
        for (idx, field) in data_struct.fields.iter().enumerate() {
            if let Some(field_ident) = &field.ident {
                seen_calls.push(quote! { self.#field_ident.seen(event_id); });
            } else {
                let index = syn::Index::from(idx);
                seen_calls.push(quote! { self.#index.seen(event_id); });
            }
        }
    }

    let mut bc_checks = Vec::new();
    let mut event_topics = Vec::new();
    for (_, ev_path) in dedup_paths_with_variant_idents(&events, "Event", events_enum_ident.span())
    {
        let ev_span = ev_path.span();
        bc_checks.push(quote::quote_spanned! { ev_span =>
            chromatofoor::bounded_context::event::assert_event_in_bc::<#scope_ty, #ev_path>();
        });
        event_topics.push(quote! {
            <#ev_path as chromatofoor::common::Kind>::TOPIC
        });
    }

    let apply_body = quote! {
        match &event.data {
            #(#match_arms,)*
        }
    };

    let seen_body = if seen_calls.is_empty() {
        quote! {
            let _ = (self, event_id);
        }
    } else {
        quote! {
            #(#seen_calls)*
        }
    };

    quote! {
        #input
        #kind_impl

        const _: fn() = || {
            #(#bc_checks)*
        };

        impl chromatofoor::bounded_context::ReadModel for #ident {
            const EVENTS: &'static [&'static str] = &[
                #(#event_topics),*
            ];
        }

        #events_enum_def

        impl chromatofoor::architecturekit::Projection for #ident {
            type Event = #events_enum_ident;
            type Error = chromatofoor::architecturekit::ViewError;

            async fn apply(
                &self,
                event: chromatofoor::architecturekit::Envelope<Self::Event>,
            ) -> ::core::result::Result<(), Self::Error> {
                #apply_body
            }

            fn seen(&self, event_id: chromatofoor::architecturekit::Revision) {
                #seen_body
            }
        }
    }
    .into()
}
