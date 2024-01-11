//! Conversions between `pavex_bp_schema` and `pavex_bp` types.
use crate::blueprint::reflection::RawCallable;
use pavex_bp_schema::{Location, RegisteredCallable};

#[track_caller]
pub(super) fn raw_callable2registered_callable(callable: RawCallable) -> RegisteredCallable {
    RegisteredCallable {
        callable: pavex_bp_schema::RawCallableIdentifiers {
            registered_at: callable.registered_at.to_owned(),
            import_path: callable.import_path.to_owned(),
        },
        location: Location::caller(),
    }
}
