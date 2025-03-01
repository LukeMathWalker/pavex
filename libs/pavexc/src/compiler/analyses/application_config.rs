use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::IndexMap;
use pavex_cli_diagnostic::{AnnotatedSnippet, CompilerDiagnostic};

use crate::{
    compiler::component::{ConfigKey, DefaultStrategy},
    diagnostic::{self, OptionalSourceSpanExt},
    language::ResolvedType,
    try_source,
    utils::comma_separated_list,
};

use super::{
    components::{ComponentDb, ComponentId, HydratedComponent},
    computations::ComputationDb,
};

/// The code-generated `struct` that holds the configurable parameters
/// for the application.
///
/// It includes all items registered via `.config()` against the
/// blueprint.
pub struct ApplicationConfig {
    bindings: BiHashMap<syn::Ident, ResolvedType>,
    binding2default: HashMap<syn::Ident, DefaultStrategy>,
}

impl ApplicationConfig {
    /// Examine the processing pipeline of all request handlers to
    /// determine which singletons are needed to serve user requests.
    pub fn new(
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut bindings = BiHashMap::new();
        let mut binding2default = HashMap::new();
        // Temporary maps to track key-to-type and type-to-key relationships
        // and detect conflicts.
        let mut key2types: BTreeMap<ConfigKey, IndexMap<ResolvedType, ComponentId>> =
            BTreeMap::new();
        let mut type2keys: IndexMap<ResolvedType, BTreeMap<ConfigKey, ComponentId>> =
            IndexMap::new();
        for (id, _) in component_db.iter() {
            let HydratedComponent::ConfigType(config) =
                component_db.hydrated_component(id, computation_db)
            else {
                continue;
            };
            key2types
                .entry(config.key().to_owned())
                .or_default()
                .insert(config.ty().to_owned(), id);
            type2keys
                .entry(config.ty().to_owned())
                .or_default()
                .insert(config.key().to_owned(), id);

            let ident = config.key().ident();
            bindings.insert(ident.clone(), config.ty().to_owned());
            binding2default.insert(ident, component_db.default_strategy(id));
        }

        for (key, type2id) in key2types {
            if type2id.len() > 1 {
                same_key_different_types(&key, &type2id, component_db, package_graph, diagnostics);
            }
        }

        for (ty, key2ids) in type2keys {
            if key2ids.len() > 1 {
                same_type_different_key(&ty, &key2ids, component_db, package_graph, diagnostics);
            }
        }

        Self {
            bindings,
            binding2default,
        }
    }

    /// Retrieve the bindings between configuration keys and their types.
    pub fn bindings(&self) -> &BiHashMap<syn::Ident, ResolvedType> {
        &self.bindings
    }

    /// Returns `true` if the field should be annotated with `#[serde(default)]`.
    pub fn should_default(&self, field_name: &syn::Ident) -> bool {
        self.binding2default[field_name] == DefaultStrategy::DefaultIfMissing
    }
}

fn same_key_different_types(
    key: &ConfigKey,
    type2id: &IndexMap<ResolvedType, ComponentId>,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let snippets: Vec<_> = type2id
        .values()
        .enumerate()
        .map(|(i, component_id)| {
            let user_component_id = component_db.user_component_id(*component_id).unwrap();
            let location = component_db
                .user_component_db()
                .get_location(user_component_id);
            try_source!(location, package_graph, diagnostics).map(|source| {
                let msg = if i == 0 {
                    "First used here..."
                } else if i == 1 {
                    "...then here"
                } else {
                    "...and here"
                };
                let label = diagnostic::get_config_key_span(&source, location).labeled(msg.into());
                AnnotatedSnippet::new_optional(source, label)
            })
        })
        // We don't want too many snippets, they'll fill the terminal viewport
        // It's enough to show the first few
        .take(4)
        .collect();
    let mut msg = format!(
        "Each configuration type must have a unique key.\n\
        `{key}` has been used as key for {} different types: ",
        type2id.len()
    );
    comma_separated_list(
        &mut msg,
        type2id.keys(),
        |t| format!("`{}`", t.display_for_error()),
        "and",
    )
    .unwrap();
    msg.push('.');
    let e = anyhow::anyhow!(msg);
    let mut diagnostic = CompilerDiagnostic::builder(e);
    for snippet in snippets {
        diagnostic = diagnostic.optional_additional_annotated_snippet(snippet);
    }
    let diagnostic = diagnostic
        .help("Choose a unique key for each configuration type.".into())
        .build();
    diagnostics.push(diagnostic.into());
}

fn same_type_different_key(
    ty: &ResolvedType,
    key2component_id: &BTreeMap<ConfigKey, ComponentId>,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let snippets: Vec<_> = key2component_id
        .values()
        .enumerate()
        .map(|(i, component_id)| {
            let user_component_id = component_db.user_component_id(*component_id).unwrap();
            let location = component_db
                .user_component_db()
                .get_location(user_component_id);
            try_source!(location, package_graph, diagnostics).map(|source| {
                let msg = if i == 0 {
                    "First used here..."
                } else if i == 1 {
                    "...then here"
                } else {
                    "...and here"
                };
                let label =
                    diagnostic::get_f_macro_invocation_span(&source, location).labeled(msg.into());
                AnnotatedSnippet::new_optional(source, label)
            })
        })
        // We don't want too many snippets, they'll fill the terminal viewport
        // It's enough to show the first few
        .take(4)
        .collect();
    let mut msg = format!(
        "A type can only appear once in the application configuration.\n\
        `{}` has been registered against {} different keys: ",
        ty.display_for_error(),
        key2component_id.len()
    );
    comma_separated_list(
        &mut msg,
        key2component_id.keys(),
        |k| format!("`{}`", k),
        "and",
    )
    .unwrap();
    msg.push('.');
    let e = anyhow::anyhow!(msg);
    let mut diagnostic = CompilerDiagnostic::builder(e);
    for snippet in snippets {
        diagnostic = diagnostic.optional_additional_annotated_snippet(snippet);
    }
    let diagnostic = diagnostic
        .help("Register the type as configuration once, with a single key.".into())
        .build();
    diagnostics.push(diagnostic.into());
}
