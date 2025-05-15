use generic::MiddlewareKind;
use proc_macro::TokenStream;

mod generic;

pub fn wrap(metadata: TokenStream, input: TokenStream) -> TokenStream {
    generic::middleware(MiddlewareKind::Wrap, metadata, input)
}

pub fn pre_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    generic::middleware(MiddlewareKind::PreProcess, metadata, input)
}

pub fn post_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    generic::middleware(MiddlewareKind::PostProcess, metadata, input)
}
