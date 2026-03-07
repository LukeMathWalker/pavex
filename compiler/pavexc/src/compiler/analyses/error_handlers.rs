use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt};
use indexmap::IndexMap;

use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::user_components::{ScopeId, UserComponentId};
use crate::compiler::component::ErrorHandler;
use crate::language::{CanonicalType, Type, TypeReference};

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
                textwrap::indent(&format!("{error_handlers:?}"), "    ")
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

    /// Record that an error handler was registered for the given type, even
    /// though the callable didn't pass our validation checks.
    pub(crate) fn insert_invalid(&mut self, error_type_ref: &Type, scope_id: ScopeId) {
        let scope_handlers = self
            .scope_id2error_handlers
            .entry(scope_id)
            .or_insert_with(ErrorHandlersInScope::new);
        scope_handlers.insert_invalid(error_type_ref);
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
        type_: &Type,
        component_db: &ComponentDb,
    ) -> Option<ErrorHandlerEntry> {
        let mut fifo = VecDeque::with_capacity(1);
        fifo.push_back(scope_id);
        while let Some(scope_id) = fifo.pop_front() {
            if let Some(handlers) = self.scope_id2error_handlers.get_mut(&scope_id)
                && let Some(output) = handlers.get_or_try_bind(type_)
            {
                return Some(output);
            }
            fifo.extend(scope_id.direct_parent_ids(component_db.scope_graph()));
        }
        None
    }
}

/// The output type of [`ErrorHandlersDb::get_or_try_bind`].
#[derive(Clone, Debug)]
pub enum ErrorHandlerEntry {
    /// There is an error handler for the given type.
    Valid {
        error_handler: ErrorHandler,
        component_id: UserComponentId,
    },
    /// An error handler for the given type was registered,
    /// but it didn't pass our validation checks.
    ///
    /// We should *not* emit a "missing error handler" diagnostic.
    Invalid,
}

/// The set of error handlers that have been registered in a given scope.
struct ErrorHandlersInScope {
    /// Map each concrete (non-templated) error type to the dedicated error handler.
    concrete: HashMap<CanonicalType, ErrorHandlerEntry>,
    /// Map each templated error type (containing unassigned generics) to the dedicated error handler.
    ///
    /// For example, if you have a `Vec<u8>`, you first look in `concrete` to see if
    /// there is an error handler for `Vec<u8>`. If there isn't, you look in
    /// `templated` to see if there is one that can match `Vec<T>`.
    ///
    /// Specialization, in a nutshell!
    templated: IndexMap<Type, ErrorHandlerEntry>,
}

impl ErrorHandlersInScope {
    /// Create a new, empty set of error handlers.
    fn new() -> Self {
        Self {
            concrete: HashMap::new(),
            templated: IndexMap::new(),
        }
    }

    /// Retrieve the handler for a given error type, if it exists.
    fn get(&self, type_: &Type) -> Option<&ErrorHandlerEntry> {
        self.concrete.get(&type_.canonicalize())
    }

    /// Retrieve the handler for a given error type, if it exists.
    ///
    /// If it doesn't exist, check the templated handlers to see if there is one
    /// that can be specialized to handle the given type.
    fn get_or_try_bind(&mut self, type_: &Type) -> Option<ErrorHandlerEntry> {
        if let Some(handler) = self.get(type_) {
            return Some(handler.to_owned());
        }
        let matched = self
            .templated
            .iter()
            .find_map(|(templated_error_type, entry)| {
                let bindings = templated_error_type.is_a_template_for(type_)?;
                Some((bindings, entry.clone()))
            });
        let (bindings, entry) = matched?;
        let (templated_error_handler, component_id) = match entry {
            ErrorHandlerEntry::Valid {
                error_handler,
                component_id,
            } => (error_handler, component_id),
            ErrorHandlerEntry::Invalid => return Some(ErrorHandlerEntry::Invalid),
        };
        let bound_handler = templated_error_handler.bind_generic_type_parameters(&bindings);
        if type_.is_a_template() {
            // The lookup type is itself a template — return the bound handler directly
            // without caching (the result is still templated).
            return Some(ErrorHandlerEntry::Valid {
                error_handler: bound_handler,
                component_id,
            });
        }
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
        let Type::Reference(TypeReference { inner, .. }) = error_type_ref else {
            unreachable!()
        };
        let error_type = *inner.to_owned();
        let entry = ErrorHandlerEntry::Valid {
            error_handler,
            component_id,
        };
        if error_type.is_a_template() {
            self.templated.insert(error_type, entry);
        } else {
            self.concrete
                .insert(error_type.canonicalize(), entry);
        }
    }

    /// Record that an error handler was registered for the given type, even
    /// though the callable didn't pass our validation checks.
    fn insert_invalid(&mut self, error_type_ref: &Type) {
        let Type::Reference(TypeReference { inner, .. }) = error_type_ref else {
            return;
        };
        let error_type = *inner.to_owned();
        if error_type.is_a_template() {
            self.templated.insert(error_type, ErrorHandlerEntry::Invalid);
        } else {
            self.concrete
                .insert(error_type.canonicalize(), ErrorHandlerEntry::Invalid);
        }
    }
}

impl std::fmt::Debug for ErrorHandlersInScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error handlers:")?;
        for (type_, entry) in &self.concrete {
            writeln!(f, "- {} -> {:?}", type_.inner().display_for_error(), entry)?;
        }
        for (type_, entry) in &self.templated {
            writeln!(f, "- {} [templated] -> {:?}", type_.display_for_error(), entry)?;
        }
        Ok(())
    }
}
