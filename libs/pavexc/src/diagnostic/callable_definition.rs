use crate::diagnostic;
use crate::diagnostic::{SourceSpanExt, convert_proc_macro_span, convert_rustdoc_span};
use crate::language::Callable;
use crate::rustdoc::CrateCollection;
use guppy::graph::PackageGraph;
use miette::{NamedSource, SourceSpan};
use pavex_cli_diagnostic::AnnotatedSource;
use rustdoc_types::ItemEnum;
use syn::spanned::Spanned;

use super::LabeledSpanExt;

/// A callable (function or method) definition,
/// parsed from the source file where it was defined.
#[allow(dead_code)]
pub struct CallableDefSource {
    pub attrs: Vec<syn::Attribute>,
    pub vis: Option<syn::Visibility>,
    pub sig: syn::Signature,
    pub block: Option<Box<syn::Block>>,
    pub span_contents: String,
    pub span_offset: usize,
    pub annotated_source: AnnotatedSource<NamedSource<String>>,
}

impl CallableDefSource {
    pub fn compute_from_item(
        item: &rustdoc_types::Item,
        package_graph: &PackageGraph,
    ) -> Option<CallableDefSource> {
        let definition_span = item.span.as_ref()?;
        let source_contents =
            diagnostic::read_source_file(&definition_span.filename, &package_graph.workspace())
                .ok()?;
        let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
        let span_offset = span.offset();
        let span_contents =
            source_contents[span.offset()..(span.offset() + span.len())].to_string();
        let (attrs, vis, sig, block) = match &item.inner {
            ItemEnum::Function(_) => {
                match syn::parse_str::<syn::ItemFn>(&span_contents) {
                    Ok(item) => (item.attrs, Some(item.vis), item.sig, Some(item.block)),
                    _ => {
                        match syn::parse_str::<syn::ImplItemFn>(&span_contents) {
                            Ok(item) => (
                                item.attrs,
                                Some(item.vis),
                                item.sig,
                                Some(Box::new(item.block)),
                            ),
                            _ => {
                                match syn::parse_str::<syn::TraitItemFn>(&span_contents) {
                                    Ok(item) => (item.attrs, None, item.sig, None),
                                    _ => {
                                        // This can happen with components defined by macros.
                                        tracing::debug!(
                                            "Could not parse as a function or method:\n{span_contents}"
                                        );
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                tracing::warn!("Expected a function or method, got: {:#?}", item.inner);
                return None;
            }
        };
        Some(CallableDefSource {
            attrs,
            vis,
            sig,
            block,
            span_contents,
            span_offset,
            annotated_source: AnnotatedSource::new(NamedSource::new(
                definition_span.filename.display().to_string(),
                source_contents,
            )),
        })
    }

    pub fn compute(
        callable: &Callable,
        krate_collection: &CrateCollection,
    ) -> Option<CallableDefSource> {
        let global_item_id = callable.source_coordinates.as_ref()?;
        let item = krate_collection.get_item_by_global_type_id(global_item_id);
        Self::compute_from_item(&item, krate_collection.package_graph())
    }

    /// Attach a label to the span of the output type for this callable.
    pub fn label_output(&mut self, label: impl Into<String>) {
        let output_span = match &self.sig.output {
            syn::ReturnType::Type(_, output_type) => output_type.span(),
            _ => self.sig.output.span(),
        };
        self.label(output_span, label);
    }

    /// Attach a label to the span of the n-th input parameter for this callable.
    pub fn label_input(&mut self, n: usize, label: impl Into<String>) {
        if let Some(input) = self.sig.inputs.iter().nth(n) {
            let input_span = input.span();
            self.label(input_span, label);
        }
    }

    /// Add a label to the span of the specified item, within the annotated source.
    ///
    /// It takes care, internally, of adjusting line/column information.
    pub fn label<Item: syn::spanned::Spanned>(&mut self, item: Item, label: impl Into<String>) {
        self.convert_local_span(item.span())
            .labeled(label.into())
            .attach(&mut self.annotated_source);
    }

    /// It makes sure to adjust line/column information.
    pub fn convert_local_span(&self, span: proc_macro2::Span) -> SourceSpan {
        convert_proc_macro_span(&self.span_contents, span).shift(self.span_offset)
    }
}
