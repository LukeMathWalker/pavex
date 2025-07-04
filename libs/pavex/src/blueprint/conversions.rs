//! Conversions between `pavex_bp_schema` and `pavex_bp` types.
use super::reflection::{AnnotationCoordinates, CreatedAt, Sources};
use crate::blueprint::Lint;
use crate::blueprint::{CloningPolicy, Lifecycle};

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

pub(super) fn created_at2created_at(created_at: CreatedAt) -> pavex_bp_schema::CreatedAt {
    pavex_bp_schema::CreatedAt {
        package_name: created_at.package_name.to_owned(),
        package_version: created_at.package_version.to_owned(),
    }
}

pub(super) fn lifecycle2lifecycle(lifecycle: Lifecycle) -> pavex_bp_schema::Lifecycle {
    match lifecycle {
        Lifecycle::RequestScoped => pavex_bp_schema::Lifecycle::RequestScoped,
        Lifecycle::Transient => pavex_bp_schema::Lifecycle::Transient,
        Lifecycle::Singleton => pavex_bp_schema::Lifecycle::Singleton,
    }
}

pub(super) fn cloning2cloning(cloning: CloningPolicy) -> pavex_bp_schema::CloningPolicy {
    match cloning {
        CloningPolicy::CloneIfNecessary => pavex_bp_schema::CloningPolicy::CloneIfNecessary,
        CloningPolicy::NeverClone => pavex_bp_schema::CloningPolicy::NeverClone,
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
