//! Conversions between `pavex_bp_schema` and `pavex_bp` types.
use crate::blueprint::constructor::{CloningStrategy, Lifecycle};
use crate::blueprint::linter::Lint;
use crate::blueprint::reflection::RawCallable;
use crate::router::AllowedMethods;
use pavex_bp_schema::{Callable, Location};
use pavex_reflection::RegisteredAt;

#[track_caller]
pub(super) fn raw_callable2registered_callable(callable: RawCallable) -> Callable {
    let registered_at = RegisteredAt {
        crate_name: callable.crate_name.to_owned(),
        module_path: callable.module_path.to_owned(),
    };
    Callable {
        callable: pavex_bp_schema::RawCallableIdentifiers {
            registered_at,
            import_path: callable.import_path.to_owned(),
        },
        location: Location::caller(),
    }
}

pub(super) fn lifecycle2lifecycle(lifecycle: Lifecycle) -> pavex_bp_schema::Lifecycle {
    match lifecycle {
        Lifecycle::RequestScoped => pavex_bp_schema::Lifecycle::RequestScoped,
        Lifecycle::Transient => pavex_bp_schema::Lifecycle::Transient,
        Lifecycle::Singleton => pavex_bp_schema::Lifecycle::Singleton,
    }
}

pub(super) fn cloning2cloning(cloning: CloningStrategy) -> pavex_bp_schema::CloningStrategy {
    match cloning {
        CloningStrategy::CloneIfNecessary => pavex_bp_schema::CloningStrategy::CloneIfNecessary,
        CloningStrategy::NeverClone => pavex_bp_schema::CloningStrategy::NeverClone,
    }
}

pub(super) fn method_guard2method_guard(
    method_guard: crate::blueprint::router::MethodGuard,
) -> pavex_bp_schema::MethodGuard {
    match method_guard.allowed_methods() {
        AllowedMethods::Some(m) => pavex_bp_schema::MethodGuard::Some(
            m.into_iter().map(|m| m.as_str().to_owned()).collect(),
        ),
        AllowedMethods::All => pavex_bp_schema::MethodGuard::Any,
    }
}

pub(super) fn lint2lint(lint: Lint) -> pavex_bp_schema::Lint {
    match lint {
        Lint::Unused => pavex_bp_schema::Lint::Unused,
    }
}
