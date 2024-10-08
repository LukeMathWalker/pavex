use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{ItemFn, Token, Visibility};

use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::processing_pipeline::pipeline::Binding;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::language::{GenericArgument, GenericLifetimeParameter, ResolvedType};

impl RequestHandlerPipeline {
    pub(crate) fn codegen(
        &self,
        pavex: &Ident,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<CodegenedRequestHandlerPipeline, anyhow::Error> {
        let id2codegened_fn = {
            let mut id2codegened_fn = IndexMap::new();
            for (&id, call_graph) in self.id2call_graph.iter() {
                let ident = &self.id2name[&id];
                if tracing::event_enabled!(tracing::Level::TRACE) {
                    call_graph.print_debug_dot(&ident, component_db, computation_db);
                }
                let fn_ = CodegenedFn {
                    fn_: {
                        let mut f =
                            call_graph.codegen(package_id2name, component_db, computation_db)?;
                        f.sig.ident = format_ident!("{}", ident);
                        f.vis = Visibility::Inherited;
                        f
                    },
                    input_parameters: call_graph.required_input_types(),
                };
                id2codegened_fn.insert(id, fn_);
            }
            id2codegened_fn
        };

        let mut stage_fns = vec![];
        for stage in &self.stages {
            let input_parameters = stage.input_parameters.clone();
            let (mut input_bindings, input_lifetimes) =
                (input_parameters.bindings, input_parameters.lifetimes);
            let response_ident = format_ident!("response");

            let ordered_by_invocation = stage
                .pre_processing_ids
                .iter()
                .copied()
                .chain(std::iter::once(stage.wrapping_id))
                .chain(stage.post_processing_ids.iter().copied())
                .collect_vec();

            if tracing::event_enabled!(tracing::Level::DEBUG) {
                let bindings = input_bindings.0.iter().fold(String::new(), |acc, binding| {
                    let mutable = binding.mutable.then(|| "mut ").unwrap_or("");
                    format!(
                        "{}\n- {}: {mutable}{:?}, ",
                        acc, binding.ident, binding.type_
                    )
                });
                let mut msg = format!("Available input bindings: {bindings}",);
                ordered_by_invocation.iter().for_each(|id| {
                    use std::fmt::Write as _;

                    let fn_ = &id2codegened_fn[id].fn_;
                    let fn_name = &fn_.sig.ident;
                    let _ = writeln!(&mut msg, "\nInput required by {fn_name}:");
                    id2codegened_fn[id]
                        .input_parameters
                        .iter()
                        .fold(&mut msg, |acc, t| {
                            let _ = writeln!(acc, "- {t:?}");
                            acc
                        });
                });
                tracing::debug!("{msg}");
            }

            let invocations = {
                let mut invocations = vec![];
                for (index, id) in ordered_by_invocation.iter().enumerate() {
                    let fn_ = &id2codegened_fn[id].fn_;
                    if component_db.is_post_processing_middleware(*id) {
                        input_bindings.0.push(Binding {
                            ident: response_ident.to_string(),
                            type_: component_db.pavex_response.clone(),
                            mutable: false,
                        });
                    }
                    let input_parameters =
                        id2codegened_fn[id]
                            .input_parameters
                            .iter()
                            .map(|input_type| {
                                match input_bindings
                                    .get_expr_for_type(input_type) {
                                    None => {
                                        let bindings = input_bindings
                                            .0
                                            .iter()
                                            .fold(String::new(), |acc, binding| {
                                                let mutable = binding.mutable.then(|| "mut ").unwrap_or("");
                                                format!("{}\n- {}: {mutable}{:?}, ", acc, binding.ident, binding.type_)
                                            });
                                        let fn_name = fn_.sig.ident.to_string();
                                        panic!(
                                            "Could not find a binding for input type `{:?}` in the input bindings \
                                            available to `{fn_name}`.\n\
                                            Input bindings: {bindings}",
                                            input_type)
                                    }
                                    Some(i) => {
                                        let mut output = i.to_token_stream();
                                        if let Some(cloning_indexes) = stage.type2cloning_indexes.get(input_type) {
                                            if cloning_indexes.contains(&index) {
                                                output = quote! { #i.clone() };
                                            }                                        }
                                        output
                                    },
                                }
                            });
                    let await_ = fn_.sig.asyncness.and_then(|_| Some(quote! { .await }));
                    let fn_name = &fn_.sig.ident;
                    let invocation = if component_db.is_pre_processing_middleware(*id) {
                        quote! {
                            if let Some(#response_ident) = #fn_name(#(#input_parameters),*)#await_.into_response() {
                                return #response_ident;
                            }
                        }
                    } else {
                        quote! {
                            let #response_ident = #fn_name(#(#input_parameters),*)#await_;
                        }
                    };
                    invocations.push(invocation);
                    if component_db.is_post_processing_middleware(*id) {
                        // Remove the response binding from the input bindings, as it's not an input parameter
                        input_bindings.0.pop();
                    }
                }
                invocations
            };

            let fn_ = {
                let asyncness = ordered_by_invocation
                    .iter()
                    .any(|id| id2codegened_fn[id].fn_.sig.asyncness.is_some())
                    .then(|| quote! { async});
                let fn_name = format_ident!("{}", stage.name);
                let visibility = if stage.name == "entrypoint" {
                    Some(Token![pub](proc_macro2::Span::call_site()))
                } else {
                    None
                };
                let input_parameters = input_bindings.0.iter().map(|input| {
                    let bound: syn::Expr = syn::parse_str(&input.ident).unwrap();
                    let ty = input.type_.syn_type(package_id2name);
                    let mut_ = input.mutable.then(|| quote! { mut });
                    quote! {
                        #mut_ #bound: #ty
                    }
                });
                let generics = input_lifetimes
                    .iter()
                    .map(|lifetime| {
                        syn::Lifetime::new(&format!("'{lifetime}"), proc_macro2::Span::call_site())
                            .to_token_stream()
                    })
                    .collect::<Vec<_>>();
                let generics = if !generics.is_empty() {
                    Some(quote! { <#(#generics),*> })
                } else {
                    None
                };
                quote! {
                    #visibility #asyncness fn #fn_name #generics(#(#input_parameters),*) -> #pavex::response::Response {
                        #(#invocations)*
                        #response_ident
                    }
                }
            };
            stage_fns.push(CodegenedFn {
                fn_: syn::parse2(fn_).unwrap(),
                input_parameters: input_bindings.0.iter().map(|i| i.type_.clone()).collect(),
            });
        }

        let next_states = {
            let mut next_states = vec![];
            for (i, stage) in self.stages[..self.stages.len() - 1].iter().enumerate() {
                let next_state = stage.next_state.as_ref().unwrap();
                let next_stage = &self.stages[i + 1];
                let bindings = next_state.field_bindings.clone();
                let next_input_types: Vec<_> = next_stage
                    .input_parameters
                    .iter()
                    .map(|input| {
                        // This is rather subtle: we're using types
                        // as they appear in the field definitions
                        // to make sure that (possible) lifetime parameters
                        // are aligned.
                        let Some(binding) = &bindings.find_exact_by_type(&input.type_) else {
                            panic!("Could not find field name for input type `{:?}` in `Next`'s state, `{:?}`", input.type_, next_state.field_bindings);
                        };
                        let ty_ = binding.type_.syn_type(package_id2name);
                        quote! { #ty_ }
                    })
                    .collect();
                let fields = next_state
                    .field_bindings
                    .0
                    .iter()
                    .map(|binding| {
                        let name = format_ident!("{}", binding.ident);
                        let ty_ = binding.type_.syn_type(package_id2name);
                        quote! {
                            #name: #ty_
                        }
                    })
                    .chain(std::iter::once(
                        quote! { next: fn(#(#next_input_types),*) -> T },
                    ));

                let struct_name = format_ident!("{}", next_state.type_.base_type.last().unwrap());
                let state_generics: Vec<_> = next_state
                    .type_
                    .generic_arguments
                    .iter()
                    .map(|arg| {
                        let GenericArgument::Lifetime(GenericLifetimeParameter::Named(lifetime)) =
                            arg
                        else {
                            unreachable!()
                        };
                        syn::Lifetime::new(&format!("'{lifetime}"), proc_macro2::Span::call_site())
                            .to_token_stream()
                    })
                    .chain(std::iter::once(quote! { T }))
                    .collect();
                let generics = quote! { <#(#state_generics),*> };
                let def = syn::parse2(quote! {
                    struct #struct_name #generics
                    where T: std::future::Future<Output = pavex::response::Response> {
                        #(#fields),*
                    }
                })
                .unwrap();
                let field_bindings = &next_state.field_bindings;
                let inputs: Vec<_> = next_stage
                    .input_parameters
                    .iter()
                    .map(|input| {
                        let Some(binding) = field_bindings.find_exact_by_type(&input.type_) else {
                            panic!("Could not find field name for input type `{:?}` in `Next`'s state, `{:?}`", input, next_state.field_bindings);
                        };
                        let ident = format_ident!("{}", binding.ident);
                        quote! {
                            self.#ident
                        }
                    })
                    .collect();
                let into_future_impl = syn::parse2(quote! {
                    impl #generics std::future::IntoFuture for #struct_name #generics
                    where
                        T: std::future::Future<Output = pavex::response::Response>,
                    {
                        type Output = pavex::response::Response;
                        type IntoFuture = T;

                        fn into_future(self) -> Self::IntoFuture {
                            (self.next)(#(#inputs),*)
                        }
                    }
                })
                .unwrap();
                next_states.push(CodegenedNextState {
                    state: def,
                    into_future_impl,
                });
            }
            next_states
        };

        Ok(CodegenedRequestHandlerPipeline {
            stages: stage_fns,
            wrapped_components: id2codegened_fn.into_values().collect(),
            next_states,
            module_name: self.module_name.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedRequestHandlerPipeline {
    /// The function that groups together the middleware/handler and (possible) post-processing
    /// invocations for each stage.
    pub(crate) stages: Vec<CodegenedFn>,
    /// The function wrapper around each middleware/handler invocation, including
    /// constructor invocations.
    pub(crate) wrapped_components: Vec<CodegenedFn>,
    /// The `Next` state for each middleware invocation.
    pub(crate) next_states: Vec<CodegenedNextState>,
    /// The name of the module that will contain the generated code.
    pub(crate) module_name: String,
}

impl CodegenedRequestHandlerPipeline {
    /// Generates an inline module containing the code generated for the pipeline
    /// of this request handler.
    pub(crate) fn as_inline_module(&self) -> TokenStream {
        let Self {
            stages,
            wrapped_components,
            next_states,
            module_name,
        } = self;
        let module_name = format_ident!("{}", module_name);
        quote! {
            pub mod #module_name {
                #(#stages)*
                #(#wrapped_components)*
                #(#next_states)*
            }
        }
    }

    /// Generates the code for invoking the first stage of the pipeline, kicking off
    /// the request processing.
    pub(crate) fn entrypoint_invocation(
        &self,
        // The name and type of each field of the application state struct.
        server_state_bindings: &BiHashMap<Ident, ResolvedType>,
        // The name and type of each field, provided by the framework, with a
        // request-scoped lifecycle.
        request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
        // The name of the variable that holds the application state.
        server_state_ident: &Ident,
    ) -> TokenStream {
        let first_stage = &self.stages[0];
        let entrypoint = &first_stage.fn_;
        let entrypoint_input_types = &first_stage.input_parameters;
        let is_handler_async = entrypoint.sig.asyncness.is_some();
        let handler_function_name = &entrypoint.sig.ident;
        let input_parameters = entrypoint_input_types.iter().map(|type_| {
            let mut is_shared_reference = false;
            let inner_type = match type_ {
                ResolvedType::Reference(r) => {
                    if !r.lifetime.is_static() {
                        is_shared_reference = true;
                        &r.inner
                    } else {
                        type_
                    }
                }
                ResolvedType::Slice(_)
                | ResolvedType::ResolvedPath(_)
                | ResolvedType::Tuple(_)
                | ResolvedType::ScalarPrimitive(_) => type_,
                ResolvedType::Generic(_) => {
                    unreachable!("Generic types should have been resolved by now")
                }
            };
            if let Some(field_name) = server_state_bindings.get_by_right(inner_type) {
                if is_shared_reference {
                    quote! {
                        &#server_state_ident.application_state.#field_name
                    }
                } else {
                    quote! {
                        #server_state_ident.application_state.#field_name.clone()
                    }
                }
            } else if let Some(field_name) = request_scoped_bindings.get_by_right(type_) {
                quote! {
                    #field_name
                }
            } else {
                let Some(field_name) = request_scoped_bindings.get_by_right(inner_type) else {
                    let rs_bindings = request_scoped_bindings
                        .iter()
                        .fold(String::new(), |acc, (ident, type_)| {
                            format!("{}\n- {}: {:?}, ", acc, ident, type_)
                        });
                    let st_bindings = server_state_bindings
                        .iter()
                        .fold(String::new(), |acc, (ident, type_)| {
                            format!("{}\n- {}: {:?}, ", acc, ident, type_)
                        });
                    panic!(
                        "Could not find a binding for input type `{:?}` in the application state or request-scoped bindings.\n\
                        Request-scoped bindings: {rs_bindings}\n\
                        Application state: {st_bindings}",
                        type_,
                    );
                };
                if is_shared_reference {
                    quote! {
                        &#field_name
                    }
                } else {
                    quote! {
                        #field_name
                    }
                }
            }
        });
        let module_name = format_ident!("{}", self.module_name);
        let mut handler_invocation =
            quote! { #module_name::#handler_function_name(#(#input_parameters),*) };
        if is_handler_async {
            handler_invocation = quote! { #handler_invocation.await };
        }
        handler_invocation
    }

    /// Returns `true` if the first stage of the pipeline (i.e. the entrypoint) needs the specified
    /// type as input.
    pub(crate) fn needs_input_type(&self, input_type: &ResolvedType) -> bool {
        self.stages[0].input_parameters.iter().any(|t| {
            if t == input_type {
                return true;
            }
            if let ResolvedType::Reference(r) = t {
                return r.inner.as_ref() == input_type;
            }

            false
        })
    }

    pub(crate) fn needs_allowed_methods(&self, framework_item_db: &FrameworkItemDb) -> bool {
        let allowed_methods_type = framework_item_db
            .get_type(FrameworkItemDb::allowed_methods_id())
            .unwrap();
        self.needs_input_type(allowed_methods_type)
    }

    pub(crate) fn needs_connection_info(&self, framework_item_db: &FrameworkItemDb) -> bool {
        let connection_info_type = framework_item_db
            .get_type(FrameworkItemDb::connection_info())
            .unwrap();
        self.needs_input_type(connection_info_type)
    }

    pub(crate) fn needs_matched_route(&self, framework_item_db: &FrameworkItemDb) -> bool {
        let matched_route_type = framework_item_db
            .get_type(FrameworkItemDb::matched_route_template_id())
            .unwrap();
        self.needs_input_type(matched_route_type)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedFn {
    pub(crate) fn_: ItemFn,
    /// We use an `IndexSet` rather than a `Vec` because we know that, due to Pavex's constraints,
    /// there won't be two input parameters with the same type.
    /// This will have to be changed if we ever support multiple input parameters with the same type.
    pub(crate) input_parameters: IndexSet<ResolvedType>,
}

impl ToTokens for CodegenedFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.fn_.to_tokens(tokens)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedNextState {
    pub(crate) state: syn::ItemStruct,
    pub(crate) into_future_impl: syn::ItemImpl,
}

impl ToTokens for CodegenedNextState {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.state.to_tokens(tokens);
        self.into_future_impl.to_tokens(tokens);
    }
}
