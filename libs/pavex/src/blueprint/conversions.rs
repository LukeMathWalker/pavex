//! Conversions between `pavex_bp_schema` and `pavex_bp` types.
use super::reflection::{AnnotationCoordinates, CreatedAt, Sources, WithLocation};
use crate::blueprint::constructor::{CloningStrategy, Lifecycle};
use crate::blueprint::linter::Lint;
use crate::blueprint::reflection::RawIdentifiers;
use pavex_bp_schema::{Callable, Location};
use pavex_reflection::CreatedBy;

#[track_caller]
pub(super) fn coordinates2coordinates(
    c: AnnotationCoordinates,
) -> pavex_reflection::AnnotationCoordinates {
    pavex_reflection::AnnotationCoordinates {
        id: c.id.to_owned(),
        created_at: created_at2created_at(c.created_at),
        macro_name: c.macro_name.to_owned(),
    }
}

#[track_caller]
pub(super) fn raw_identifiers2callable(callable: WithLocation<RawIdentifiers>) -> Callable {
    let WithLocation {
        value: callable,
        created_at,
    } = callable;
    if callable.macro_name == "t" {
        panic!(
            "You need to use the `f!` macro to register function-like components (e.g. a constructor).\n\
            Here you used the `t!` macro, which is reserved type-like components, like state inputs."
        )
    }
    Callable {
        callable: pavex_bp_schema::RawIdentifiers {
            created_at: created_at2created_at(created_at),
            created_by: CreatedBy::macro_name(callable.macro_name),
            import_path: callable.import_path.to_owned(),
        },
        registered_at: Location::caller(),
    }
}

pub(super) fn created_at2created_at(created_at: CreatedAt) -> pavex_bp_schema::CreatedAt {
    pavex_bp_schema::CreatedAt {
        package_name: created_at.package_name.to_owned(),
        package_version: created_at.package_version.to_owned(),
        module_path: created_at.module_path.to_owned(),
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

pub(super) fn lint2lint(lint: Lint) -> pavex_bp_schema::Lint {
    match lint {
        Lint::Unused => pavex_bp_schema::Lint::Unused,
    }
}

pub(super) fn sources2sources(sources: Sources) -> pavex_bp_schema::Sources {
    match sources {
        Sources::All => pavex_bp_schema::Sources::All,
        Sources::Some(s) => {
            pavex_bp_schema::Sources::Some(s.into_iter().map(|s| s.into_owned()).collect())
        }
    }
}
