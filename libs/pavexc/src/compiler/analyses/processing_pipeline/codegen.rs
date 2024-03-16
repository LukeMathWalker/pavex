use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::ItemFn;

use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::language::{GenericArgument, GenericLifetimeParameter, ResolvedType};

impl RequestHandlerPipeline {
    /// Generates the code required to wire together this request handler pipeline.
    ///
    /// This method generates the code for the following:
    /// - The closure of the request handler function
    /// - The closure of each middleware functions
    /// - The `Next` state for each middleware invocation
    ///
    /// You can wrap the generated code in an inline module by calling the
    /// [`as_inline_module`](CodegenedRequestHandlerPipeline::as_inline_module) method on
    /// the output.
    pub(crate) fn codegen(
        &self,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<CodegenedRequestHandlerPipeline, anyhow::Error> {
        let n_middlewares = self.middleware_id2stage_data.len();
        let mut stages = Vec::with_capacity(n_middlewares + 1);
        for (i, call_graph) in self.graph_iter().enumerate() {
            let ident = if i < n_middlewares {
                format_ident!("middleware_{}", i)
            } else {
                format_ident!("handler")
            };
            if tracing::event_enabled!(tracing::Level::TRACE) {
                call_graph.print_debug_dot(component_db, computation_db);
            }
            let mut fn_ = call_graph.codegen(package_id2name, component_db, computation_db)?;
            fn_.sig.ident = ident;
            let stage = CodegenedFn {
                fn_,
                input_parameters: call_graph.required_input_types(),
            };
            stages.push(stage);
        }

        let mut next_states = Vec::with_capacity(n_middlewares);
        for (i, stage_data) in self.middleware_id2stage_data.values().enumerate() {
            let next_state = &stage_data.next_state;
            let next_stage = &stages[i + 1];
            let input_types: Vec<_> = next_stage
                .input_parameters
                .iter()
                .map(|input| {
                    // This is rather subtle: we're using types
                    // as they appears in the field definitions
                    // to make sure that (possible) lifetime parameters
                    // are aligned.
                    let field = next_state
                        .field_bindings
                        .iter()
                        .find(|(_, ty_)| *ty_ == input);
                    let Some((_, ty_)) = field else {
                        unreachable!();
                    };
                    let ty_ = ty_.syn_type(package_id2name);
                    quote! { #ty_ }
                })
                .collect();
            let fields = next_state
                .field_bindings
                .iter()
                .map(|(name, ty_)| {
                    let name = format_ident!("{}", name);
                    let ty_ = ty_.syn_type(package_id2name);
                    quote! {
                        #name: #ty_
                    }
                })
                .chain(std::iter::once(quote! { next: fn(#(#input_types),*) -> T }));

            let struct_name = format_ident!("{}", next_state.type_.base_type.last().unwrap());
            let state_generics: Vec<_> = next_state
                .type_
                .generic_arguments
                .iter()
                .map(|arg| {
                    let GenericArgument::Lifetime(GenericLifetimeParameter::Named(lifetime)) = arg
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
                pub struct #struct_name #generics
                where T: std::future::Future<Output = pavex::response::Response> {
                    #(#fields),*
                }
            })
            .unwrap();
            let inputs: Vec<_> = next_stage
                .input_parameters
                .iter()
                .map(|input| {
                    let field_name = next_state
                        .field_bindings
                        .iter()
                        .find(|(_, ty_)| ty_ == &input);
                    if let Some((field_name, _)) = field_name {
                        let ident = format_ident!("{}", field_name);
                        quote! {
                            self.#ident
                        }
                    } else if let ResolvedType::Reference(r) = input {
                        let field_name = next_state
                            .field_bindings
                            .iter()
                            .find(|(_, ty_)| *ty_ == r.inner.as_ref())
                            .unwrap()
                            .0;
                        let ident = format_ident!("{}", field_name);
                        quote! {
                                &self.#ident
                            }
                    } else {
                        panic!("Could not find field name for input type `{:?}` in `Next`'s state, `{:?}`", input, next_state.field_bindings);
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

        Ok(CodegenedRequestHandlerPipeline {
            stages,
            next_states,
            module_name: self.module_name.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedRequestHandlerPipeline {
    /// The closure for each stage (i.e. middleware or request handler) of the pipeline.
    pub(crate) stages: Vec<CodegenedFn>,
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
            next_states,
            module_name,
        } = self;
        let module_name = format_ident!("{}", module_name);
        quote! {
            pub mod #module_name {
                #(#stages)*
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
                let field_name = request_scoped_bindings.get_by_right(inner_type).unwrap();
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
