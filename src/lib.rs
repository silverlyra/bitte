use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Ident, ItemFn, ItemImpl, ItemTrait, Meta, ReturnType, Signature, Token, TraitItem,
    TraitItemFn, Type, TypeImplTrait, TypeParamBound, WherePredicate, ImplItem, ImplItemFn,
    parse::{Parse, ParseStream},
    parse_quote, FnArg, PathArguments, GenericArgument,
};

/// Whether to bound an `async fn` or its receiver by [`Send`] or [`Sync`].
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum AsyncBound {
    Send(bool),
    Sync(bool),
}

impl Parse for AsyncBound {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let enabled = if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            false
        } else {
            true
        };

        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Send" => Ok(AsyncBound::Send(enabled)),
            "Sync" => Ok(AsyncBound::Sync(enabled)),
            _ => Err(syn::Error::new_spanned(ident, "Expected Send or Sync")),
        }
    }
}

/// Whether to bound an `async fn`’s [`Future`] by [`Send`] or
/// its receiver by [`Sync`].
#[derive(Debug, Clone)]
struct AsyncBounds {
    send: bool,
    sync: bool,
}

impl Default for AsyncBounds {
    fn default() -> Self {
        AsyncBounds {
            send: cfg!(feature = "threads"),
            sync: cfg!(feature = "threads"),
        }
    }
}

impl AsyncBounds {
    fn from_attribute(attr: &Attribute) -> syn::Result<Self> {
        let mut config = AsyncBounds::default();

        if let Meta::List(meta_list) = &attr.meta {
            let parsed = meta_list.parse_args_with(
                syn::punctuated::Punctuated::<AsyncBound, syn::Token![,]>::parse_terminated,
            )?;

            for arg in parsed {
                match arg {
                    AsyncBound::Send(b) => config.send = b,
                    AsyncBound::Sync(b) => config.sync = b,
                }
            }
        }

        Ok(config)
    }
}

/// Apply the bitte transformation to a trait, impl block, or async function
///
/// # Examples
///
/// Apply to an entire trait:
/// ```rust
/// use bitte::bitte;
///
/// #[bitte]
/// trait AsyncTrait {
///     async fn method(&self) -> u32;
/// }
/// ```
///
/// Apply to an impl block to write natural async methods:
/// ```rust
/// use bitte::bitte;
///
/// #[bitte]
/// trait AsyncTrait {
///     async fn method(&self) -> u32;
/// }
///
/// struct MyStruct;
///
/// #[bitte]
/// impl AsyncTrait for MyStruct {
///     async fn method(&self) -> u32 {
///         42
///     }
/// }
/// ```
///
/// Apply to individual methods with custom bounds:
/// ```rust
/// use bitte::bitte;
///
/// trait AsyncTrait {
///     #[bitte(?Send)]
///     async fn method(&self) -> u32;
/// }
/// ```
#[proc_macro_attribute]
pub fn bitte(args: TokenStream, input: TokenStream) -> TokenStream {
    let config = if args.is_empty() {
        AsyncBounds::default()
    } else {
        let args = proc_macro2::TokenStream::from(args);
        let attr: Attribute = parse_quote! { #[bitte(#args)] };
        match AsyncBounds::from_attribute(&attr) {
            Ok(config) => config,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    // Try to parse as a trait first
    if let Ok(mut trait_item) = syn::parse::<ItemTrait>(input.clone()) {
        return trait_item.desugar_async(&config).into();
    }

    // Try to parse as an impl block
    if let Ok(mut impl_item) = syn::parse::<ItemImpl>(input.clone()) {
        return impl_item.desugar_async(&config).into();
    }

    // Try to parse as a function
    if let Ok(mut fn_item) = syn::parse::<ItemFn>(input.clone()) {
        return fn_item.desugar_async(&config).into();
    }

    // If neither, try parsing as a trait item function
    if let Ok(mut trait_fn) = syn::parse::<TraitItemFn>(input) {
        return trait_fn.desugar_async(&config).into();
    }

    syn::Error::new(
        Span::call_site(),
        "bitte can only be applied to traits, impl blocks, functions, or trait methods",
    )
    .to_compile_error()
    .into()
}

trait DesugarAsync {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream;
}

impl DesugarAsync for ItemTrait {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        for item in &mut self.items {
            if let TraitItem::Fn(method) = item {
                if method.sig.asyncness.is_some() {
                    method.desugar_async(config);
                }
            }
        }
        quote! { #self }
    }
}

impl DesugarAsync for ItemImpl {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        for item in &mut self.items {
            if let ImplItem::Fn(method) = item {
                if method.sig.asyncness.is_some() {
                    method.desugar_async(config);
                }
            }
        }
        quote! { #self }
    }
}

impl DesugarAsync for ItemFn {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        if self.sig.asyncness.is_some() {
            self.sig.desugar_async(config);
            // Add #[must_use] attribute to async functions
            self.attrs.push(parse_quote! { #[must_use] });
            // Add lint suppression
            self.attrs.push(parse_quote! {
                #[allow(
                    clippy::type_complexity,
                    clippy::type_repetition_in_bounds
                )]
            });
        }
        quote! { #self }
    }
}

impl DesugarAsync for ImplItemFn {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        if self.sig.asyncness.is_some() {
            // Store the original body
            let body = &self.block;
            
            // Transform the signature
            self.sig.desugar_async(config);
            
            // Wrap the body in an async block
            self.block = parse_quote! {
                {
                    async move #body
                }
            };
            
            // Add #[must_use] attribute to async methods
            self.attrs.push(parse_quote! { #[must_use] });
            // Add lint suppression
            self.attrs.push(parse_quote! {
                #[allow(
                    clippy::async_yields_async,
                    clippy::let_unit_value,
                    clippy::no_effect_underscore_binding,
                    clippy::shadow_same,
                    clippy::type_complexity,
                    clippy::type_repetition_in_bounds,
                    clippy::used_underscore_binding
                )]
            });
        }
        quote! { #self }
    }
}

impl DesugarAsync for TraitItemFn {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        if self.sig.asyncness.is_some() {
            self.sig.desugar_async(config);
            // Add #[must_use] attribute to async methods
            self.attrs.push(parse_quote! { #[must_use] });
            // Add lint suppression
            let lint_attr = if self.default.is_some() {
                // With default implementation
                parse_quote! {
                    #[allow(
                        clippy::async_yields_async,
                        clippy::let_unit_value,
                        clippy::no_effect_underscore_binding,
                        clippy::shadow_same,
                        clippy::type_complexity,
                        clippy::type_repetition_in_bounds,
                        clippy::used_underscore_binding
                    )]
                }
            } else {
                // Without default implementation
                parse_quote! {
                    #[allow(
                        clippy::type_complexity,
                        clippy::type_repetition_in_bounds
                    )]
                }
            };
            self.attrs.push(lint_attr);
            
            // Transform default method body if present
            if let Some(block) = &mut self.default {
                let transformed = quote! {
                    {
                        async move #block
                    }
                };
                self.default = Some(parse_quote! { #transformed });
            }
        }
        quote! { #self }
    }
}

impl DesugarAsync for Signature {
    fn desugar_async(&mut self, config: &AsyncBounds) -> proc_macro2::TokenStream {
        // Remove the async keyword
        self.asyncness = None;

        // Extract the original return type
        let output_type = match &self.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        };

        // Build the Future bounds
        let mut bounds: Vec<TypeParamBound> =
            vec![parse_quote! { std::future::Future<Output = #output_type> }];
        
        // Check receiver type to determine bounds
        let receiver_bounds = analyze_receiver(&self.inputs);
        
        if config.send || receiver_bounds.needs_send {
            bounds.push(parse_quote! { Send });
        }


        // Create the new return type
        let impl_trait = TypeImplTrait {
            impl_token: syn::token::Impl::default(),
            bounds: bounds.into_iter().collect(),
        };

        self.output = ReturnType::Type(
            syn::token::RArrow::default(),
            Box::new(Type::ImplTrait(impl_trait)),
        );

        // Add Self: Sync bound if needed
        if config.sync || receiver_bounds.needs_sync {
            add_self_sync_bound(self);
        }

        quote! { #self }
    }
}

struct ReceiverBounds {
    needs_send: bool,
    needs_sync: bool,
}

fn analyze_receiver(inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>) -> ReceiverBounds {
    if let Some(FnArg::Receiver(receiver)) = inputs.first() {
        match &*receiver.ty {
            // Arc<Self> requires both Send and Sync
            Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Arc" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            if args.args.len() == 1 {
                                if let GenericArgument::Type(Type::Path(inner)) = &args.args[0] {
                                    if inner.path.is_ident("Self") {
                                        return ReceiverBounds {
                                            needs_send: true,
                                            needs_sync: true,
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // &self requires only Sync
            Type::Reference(type_ref) if type_ref.mutability.is_none() => {
                return ReceiverBounds {
                    needs_send: false,
                    needs_sync: true,
                };
            }
            // Other receiver types (self, &mut self) require Send
            _ => {
                return ReceiverBounds {
                    needs_send: true,
                    needs_sync: false,
                };
            }
        }
    }
    
    ReceiverBounds {
        needs_send: false,
        needs_sync: false,
    }
}

fn add_self_sync_bound(sig: &mut Signature) {
    let sync_bound: WherePredicate = parse_quote! { Self: Sync };

    if sig.generics.where_clause.is_none() {
        sig.generics.where_clause = Some(parse_quote! { where });
    }

    sig.generics
        .where_clause
        .as_mut()
        .unwrap()
        .predicates
        .push(sync_bound);
}


