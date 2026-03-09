//! Pavex-specific cache integration for rustdoc.
mod cache;

pub(super) use cache::CacheEntryExt;
pub(crate) use cache::{RustdocCacheKey, pavex_rustdoc_cache};
