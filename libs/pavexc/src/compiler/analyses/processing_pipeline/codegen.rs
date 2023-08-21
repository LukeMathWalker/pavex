use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::language::ResolvedType;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::ItemFn;

impl RequestHandlerPipeline {
    pub(crate) fn codegen(
        &self,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<CodegenedRequestHandlerPipeline, anyhow::Error> {
        let n_middlewares = self.middleware_id2stage_data.len();
        let mut stages = Vec::with_capacity(n_middlewares + 1);
        for (i, call_graph) in self.graph_iter().enumerate() {
            let mut fn_ = call_graph.codegen(package_id2name, component_db, computation_db)?;
            fn_.sig.ident = if i < n_middlewares {
                format_ident!("middleware_{}", i)
            } else {
                format_ident!("handler")
            };
            let stage = CodegenedFn {
                fn_,
                input_parameters: call_graph.required_input_types(),
            };
            stages.push(stage);
        }

        // struct Next0 {
        //     rs_0: pavex::request::RequestHead,
        // }
        //
        // impl std::future::IntoFuture for Next0 {
        //     type Output = pavex::response::Response;
        //     type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output>>>;
        //
        //     fn into_future(self) -> Self::IntoFuture {
        //         Box::pin(async {
        //             let x = todo!();
        //             handler(x, self.rs_0).await
        //         })
        //     }
        // }

        Ok(CodegenedRequestHandlerPipeline {
            stages,
            next_states: vec![],
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedRequestHandlerPipeline {
    pub(crate) stages: Vec<CodegenedFn>,
    pub(crate) next_states: Vec<CodegenedNextState>,
}

impl CodegenedRequestHandlerPipeline {
    /// Generates an inline module containing the code generated for the pipeline
    /// of this request handler.
    pub(crate) fn as_inline_module(&self, module_name: Ident) -> TokenStream {
        let Self {
            stages,
            next_states,
        } = self;
        quote! {
            pub mod #module_name {
                #(#stages)*
                #(#next_states)*
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedFn {
    pub(crate) fn_: ItemFn,
    /// We use an `IndexSet` rather than a `Vec` because we know that, due to the Pavex's constraints,
    /// there won't be two input parameters with the same type.
    /// This will have to be changed if we ever support multiple input parameters with the same type.
    pub(crate) input_parameters: IndexSet<ResolvedType>,
}

impl CodegenedFn {
    pub(crate) fn invocation(
        &self,
        module_name: &Ident,
        singleton_bindings: &BiHashMap<Ident, ResolvedType>,
        request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
        server_state_ident: &Ident,
    ) -> TokenStream {
        let handler = &self.fn_;
        let handler_input_types = &self.input_parameters;
        let is_handler_async = handler.sig.asyncness.is_some();
        let handler_function_name = &handler.sig.ident;
        let input_parameters = handler_input_types.iter().map(|type_| {
            let mut is_shared_reference = false;
            let inner_type = match type_ {
                ResolvedType::Reference(r) => {
                    if !r.is_static {
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
            if let Some(field_name) = singleton_bindings.get_by_right(inner_type) {
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
        let mut handler_invocation =
            quote! { #module_name::#handler_function_name(#(#input_parameters),*) };
        if is_handler_async {
            handler_invocation = quote! { #handler_invocation.await };
        }
        handler_invocation
    }
}

impl ToTokens for CodegenedFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.fn_.to_tokens(tokens)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CodegenedNextState {
    pub(crate) state: syn::ItemStruct,
    pub(crate) future_impl_: syn::ItemImpl,
}

impl ToTokens for CodegenedNextState {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.state.to_tokens(tokens);
        self.future_impl_.to_tokens(tokens);
    }
}
