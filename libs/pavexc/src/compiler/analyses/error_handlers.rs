use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt};
use indexmap::{IndexMap, IndexSet};

use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::user_components::{ScopeId, UserComponentId};
use crate::compiler::component::ErrorHandler;
use crate::language::{ResolvedType, TypeReference};

/// The set of error types that can be handled, for each scope.
#[derive(Default)]
pub(crate) struct ErrorHandlersDb {
    scope_id2error_handlers: IndexMap<ScopeId, ErrorHandlersInScope>,
}

impl std::fmt::Debug for ErrorHandlersDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Available error handlers:\n")?;
        for (scope_id, error_handlers) in &self.scope_id2error_handlers {
            writeln!(
                f,
                "- {scope_id}:\n{}",
                // TODO: Use a PadAdapter down here to avoid allocating an intermediate string
                textwrap::indent(&format!("{:?}", error_handlers), "    ")
            )?;
        }
        Ok(())
    }
}

impl ErrorHandlersDb {
    /// Add a new error handler to the database.
    pub(crate) fn insert(
        &mut self,
        error_handler: ErrorHandler,
        scope_id: ScopeId,
        id: UserComponentId,
    ) {
        let scope_handlers = self
            .scope_id2error_handlers
            .entry(scope_id)
            .or_insert_with(ErrorHandlersInScope::new);
        scope_handlers.insert(error_handler, id);
    }

    /// Find the error handler for a given error type in a given scope.
    ///
    /// If the error type has no handler in the given scope, we look for a handler in the
    /// parent scope, and so on until we reach the root scope.
    /// If we reach the root scope and the type still doesn't have a handler, we return `None`.
    ///
    /// It also inspects templated types to see if they can be instantiated in such a way to build
    /// the error type that we want to handle.
    /// If that's the case, we bind the generic error handler, add it to the database and return
    /// the id of the newly bound transformer.
    pub(crate) fn get_or_try_bind(
        &mut self,
        scope_id: ScopeId,
        type_: &ResolvedType,
        component_db: &ComponentDb,
    ) -> Option<(ErrorHandler, UserComponentId)> {
        let mut fifo = VecDeque::with_capacity(1);
        fifo.push_back(scope_id);
        while let Some(scope_id) = fifo.pop_front() {
            if let Some(handlers) = self.scope_id2error_handlers.get_mut(&scope_id) {
                if let Some(output) = handlers.get_or_try_bind(type_) {
                    return Some(output);
                }
            }
            fifo.extend(scope_id.direct_parent_ids(component_db.scope_graph()));
        }
        None
    }
}

/// The set of constructibles that have been registered in a given scope.
///
/// Be careful! This is not the set of all types that can be constructed in the given scope!
/// That's a much larger set, because it includes all types that can be constructed in this
/// scope as well as any of its parent scopes.
struct ErrorHandlersInScope {
    /// Map each supported error type to the dedicated error handler.
    type2handler: HashMap<ResolvedType, (ErrorHandler, UserComponentId)>,
    /// Every time we encounter an error type that contains an unassigned generic type
    /// (e.g. `T` in `Vec<T>` instead of `u8` in `Vec<u8>`), we store it here.
    ///
    /// For example, if you have a `Vec<u8>`, you first look in `type2handler` to see if
    /// there is an error handler for `Vec<u8>`. If there isn't, you look in
    /// `templated` to see if there is one that can match `Vec<T>`.
    ///
    /// Specialization, in a nutshell!
    templated: IndexSet<ResolvedType>,
}

impl ErrorHandlersInScope {
    /// Create a new, empty set of constructibles.
    fn new() -> Self {
        Self {
            type2handler: HashMap::new(),
            templated: IndexSet::new(),
        }
    }

    /// Retrieve the handler for a given error type, if it exists.
    fn get(&self, type_: &ResolvedType) -> Option<&(ErrorHandler, UserComponentId)> {
        self.type2handler.get(type_)
    }

    /// Retrieve the handler for a given error type, if it exists.
    ///
    /// If it doesn't exist, check the templated handlers to see if there is one
    /// that can be specialized to handle the given type.
    fn get_or_try_bind(&mut self, type_: &ResolvedType) -> Option<(ErrorHandler, UserComponentId)> {
        if let Some(handler) = self.get(type_) {
            return Some(handler.to_owned());
        }
        let (bindings, templated_error_type) =
            self.templated.iter().find_map(|templated_error_type| {
                let bindings = templated_error_type.is_a_template_for(type_)?;
                Some((bindings, templated_error_type))
            })?;
        let (templated_error_handler, component_id) =
            self.get(templated_error_type).cloned().unwrap();
        let bound_handler = templated_error_handler.bind_generic_type_parameters(&bindings);
        self.insert(bound_handler, component_id);
        let bound = self.get(type_);
        assert!(
            bound.is_some(),
            "I used {:?} as a templated error handler to build {} but the binding process didn't succeed as expected.\nBindings:\n{}",
            templated_error_handler,
            type_.display_for_error(),
            bindings
                .into_iter()
                .map(|(k, v)| format!("- {k} -> {}", v.display_for_error()))
                .collect::<Vec<_>>()
                .join("\n")
        );
        bound.cloned()
    }

    /// Register a type and its handler.
    fn insert(&mut self, error_handler: ErrorHandler, component_id: UserComponentId) {
        let error_type_ref = error_handler.error_type_ref();
        let ResolvedType::Reference(TypeReference { inner, .. }) = error_type_ref else {
            unreachable!()
        };
        let error_type = *inner.to_owned();
        if error_type.is_a_template() {
            self.templated.insert(error_type.clone());
        }
        self.type2handler
            .insert(error_type, (error_handler, component_id));
    }
}

impl std::fmt::Debug for ErrorHandlersInScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error handlers:")?;
        for (type_, component_id) in &self.type2handler {
            writeln!(f, "- {} -> {:?}", type_.display_for_error(), component_id)?;
        }
        Ok(())
    }
}
