use crate::diagnostic;
use crate::diagnostic::convert_rustdoc_span;
use crate::language::Callable;
use crate::rustdoc::CrateCollection;
use guppy::graph::PackageGraph;
use miette::NamedSource;
use rustdoc_types::ItemEnum;
use std::path::PathBuf;

/// A callable (function or method) definition,
/// parsed from the source file where it was defined.
pub struct CallableDefinition {
    pub attrs: Vec<syn::Attribute>,
    pub vis: syn::Visibility,
    pub sig: syn::Signature,
    pub block: Box<syn::Block>,
    pub span_contents: String,
    pub source_file: PathBuf,
}

impl CallableDefinition {
    pub fn compute(
        callable: &Callable,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
    ) -> Option<CallableDefinition> {
        let global_item_id = callable.source_coordinates.as_ref()?;
        let item = krate_collection.get_type_by_global_type_id(global_item_id);
        let definition_span = item.span.as_ref()?;
        let source_contents =
            diagnostic::read_source_file(&definition_span.filename, &package_graph.workspace())
                .ok()?;
        let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
        let span_contents =
            source_contents[span.offset()..(span.offset() + span.len())].to_string();
        let (attrs, vis, sig, block) = match &item.inner {
            ItemEnum::Function(_) => {
                if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                    (item.attrs, item.vis, item.sig, item.block)
                } else if let Ok(item) = syn::parse_str::<syn::ImplItemFn>(&span_contents) {
                    (item.attrs, item.vis, item.sig, Box::new(item.block))
                } else {
                    // TODO: convert into miette diagnostics
                    panic!("Could not parse as a function or method:\n{span_contents}")
                }
            }
            _ => unreachable!(),
        };
        Some(CallableDefinition {
            attrs,
            vis,
            sig,
            block,
            span_contents,
            source_file: definition_span.filename.clone(),
        })
    }

    pub fn named_source(&self) -> NamedSource<String> {
        NamedSource::new(
            self.source_file.display().to_string(),
            self.span_contents.clone(),
        )
    }
}
