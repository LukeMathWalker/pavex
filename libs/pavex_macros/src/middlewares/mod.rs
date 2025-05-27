use generic::MiddlewareKind;

use crate::{
    fn_like::{Callable, CallableAnnotation, ImplContext},
    utils::AnnotationCodegen,
};

mod generic;

pub struct WrapAnnotation;

impl CallableAnnotation for WrapAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Middlewares";

    const ATTRIBUTE: &str = "#[pavex::wrap]";

    type InputSchema = generic::InputSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        generic::middleware(MiddlewareKind::Wrap, metadata, item)
    }
}

pub struct PreProcessAnnotation;

impl CallableAnnotation for PreProcessAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Middlewares";

    const ATTRIBUTE: &str = "#[pavex::pre_process]";

    type InputSchema = generic::InputSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        generic::middleware(MiddlewareKind::PreProcess, metadata, item)
    }
}

pub struct PostProcessAnnotation;

impl CallableAnnotation for PostProcessAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Middlewares";

    const ATTRIBUTE: &str = "#[pavex::post_process]";

    type InputSchema = generic::InputSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        generic::middleware(MiddlewareKind::PostProcess, metadata, item)
    }
}
