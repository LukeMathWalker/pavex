use std::collections::BTreeMap;

use bimap::{BiBTreeMap, BiHashMap};
use indexmap::IndexMap;
use miette::Severity;
use pavex_bp_schema::{Lint, LintSetting};
use pavex_cli_diagnostic::CompilerDiagnostic;

use crate::{
    compiler::component::{ConfigKey, DefaultStrategy},
    diagnostic::{self, TargetSpan},
    language::ResolvedType,
    utils::comma_separated_list,
};

use super::{
    call_graph::ApplicationStateCallGraph,
    components::{ComponentDb, ComponentId, HydratedComponent},
    computations::ComputationDb,
    processing_pipeline::RequestHandlerPipeline,
};

/// The code-generated `struct` that holds the configurable parameters
/// for the application.
///
/// It includes all items registered via `.config()` against the
/// blueprint.
pub struct ApplicationConfig {
    bindings: BiHashMap<syn::Ident, ResolvedType>,
    binding2default: BTreeMap<syn::Ident, DefaultStrategy>,
    binding2id: BiBTreeMap<syn::Ident, ComponentId>,
}

impl ApplicationConfig {
    /// Retrieve all registered configuration types.
    ///
    /// Ensure that configuration types are unique and that each type is
    /// associated with an equally unique configuration key.
    pub fn new(
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) -> Self {
        let mut bindings = BiHashMap::new();
        let mut binding2default = BTreeMap::new();
        let mut binding2id = BiBTreeMap::new();
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
            binding2default.insert(ident.clone(), component_db.default_strategy(id));
            binding2id.insert(ident, id);
        }

        for (key, type2id) in key2types {
            if type2id.len() > 1 {
                same_key_different_types(&key, &type2id, component_db, diagnostics);
            }
        }

        for (ty, key2ids) in type2keys {
            if key2ids.len() > 1 {
                same_type_different_key(&ty, &key2ids, component_db, diagnostics);
            }
        }

        Self {
            bindings,
            binding2default,
            binding2id,
        }
    }

    /// Remove unused configuration types from the bindings.
    ///
    /// A warning is issued for each pruned configuration type.
    pub fn prune_unused(
        &mut self,
        handler_id2pipeline: &IndexMap<ComponentId, RequestHandlerPipeline>,
        state_call_graph: &ApplicationStateCallGraph,
        db: &ComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        // Configuration type ids are not directly used in call graphs.
        // We need to look for the corresponding (synthetic) constructor ids.
        let mut constructor_id2config_id = self
            .binding2id
            .right_values()
            .map(|id| (db.config2constructor(*id), *id))
            .collect::<BiBTreeMap<_, _>>();
        for node in handler_id2pipeline
            .values()
            .flat_map(|p| p.graph_iter())
            .flat_map(|g| g.call_graph.node_weights())
            .chain(state_call_graph.call_graph.call_graph.node_weights())
        {
            let Some(id) = node.component_id() else {
                continue;
            };
            if constructor_id2config_id.remove_by_left(&id).is_some()
                && constructor_id2config_id.is_empty()
            {
                // All config types are used, no point to keep iterating
                // over the remaining nodes.
                return;
            }
        }
        for config_id in constructor_id2config_id.right_values() {
            if db.include_if_unused(*config_id) {
                continue;
            }
            let (_, id, ty) = self.remove(*config_id);

            // Should the issue be reported?
            if let Some(lints) = db.lints(id) {
                if let Some(LintSetting::Allow) = lints.get(&Lint::Unused) {
                    continue;
                }
            }

            unused_configuration_type(id, &ty, db, diagnostics);
        }
    }

    /// Remove a binding from the generated `ApplicationConfig` type.
    ///
    /// Panics if there is binding with the given id.
    fn remove(&mut self, id: ComponentId) -> (syn::Ident, ComponentId, ResolvedType) {
        let (ident, id) = self.binding2id.remove_by_right(&id).unwrap();
        let (_, ty) = self.bindings.remove_by_left(&ident).unwrap();
        self.binding2default.remove(&ident);
        (ident, id, ty)
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
    db: &ComponentDb,
    diagnostics: &diagnostic::DiagnosticSink,
) {
    let mut counter = 0;
    let snippets: Vec<_> = type2id
        .values()
        .map(|id| {
            let user_id = db.user_component_id(*id)?;
            let msg = if counter == 0 {
                "First used here..."
            } else if counter == 1 {
                "...then here"
            } else {
                "...and here"
            };
            let s = diagnostics.annotated(TargetSpan::ConfigKeySpan(db.registration(user_id)), msg);
            if s.is_some() {
                counter += 1;
            }
            s
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
        diagnostic = diagnostic.optional_source(snippet);
    }
    let diagnostic = diagnostic
        .help("Choose a unique key for each configuration type.".into())
        .build();
    diagnostics.push(diagnostic);
}

fn same_type_different_key(
    ty: &ResolvedType,
    key2component_id: &BTreeMap<ConfigKey, ComponentId>,
    db: &ComponentDb,
    diagnostics: &diagnostic::DiagnosticSink,
) {
    let mut counter = 0;
    let snippets: Vec<_> = key2component_id
        .values()
        .map(|id| {
            let user_id = db.user_component_id(*id)?;
            let msg = if counter == 0 {
                "First used here..."
            } else if counter == 1 {
                "...then here"
            } else {
                "...and here"
            };
            let s = diagnostics.annotated(db.registration_target(user_id), msg);
            if s.is_some() {
                counter += 1;
            }
            s
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
        |k| format!("`{k}`"),
        "and",
    )
    .unwrap();
    msg.push('.');
    let e = anyhow::anyhow!(msg);
    let mut diagnostic = CompilerDiagnostic::builder(e);
    for snippet in snippets {
        diagnostic = diagnostic.optional_source(snippet);
    }
    let diagnostic = diagnostic
        .help("Register the type as configuration once, with a single key.".into())
        .build();
    diagnostics.push(diagnostic);
}

fn unused_configuration_type(
    id: ComponentId,
    ty: &ResolvedType,
    db: &ComponentDb,
    diagnostics: &diagnostic::DiagnosticSink,
) {
    let user_id = db.user_component_id(id).unwrap();
    let s = diagnostics.annotated(db.registration_target(user_id), "Registered here");
    let ty = ty.display_for_error();
    let e = anyhow::anyhow!(
        "`{ty}` is never used.\n\
        `{ty}` is registered as a configuration type, but it is never injected as an input parameter. \
        Pavex won't include it in the generated `ApplicationConfig`."
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(s)
        .severity(Severity::Warning)
        .help(format!("Use `include_if_unused` if you want to force Pavex to include a `{ty}` field in `ApplicationConfig`, even if it's not used."))
        .build();
    diagnostics.push(diagnostic);
}
