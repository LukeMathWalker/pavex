//! Types for serialized cache entries.

use std::borrow::Cow;

use super::utils::RkyvCowBytes;

/// Data that can be computed starting from the raw JSON documentation for a crate,
/// without having to re-invoke `rustdoc`.
#[derive(Debug)]
pub struct SecondaryIndexes<'a> {
    pub import_index: Cow<'a, [u8]>,
    pub annotated_items: Option<Cow<'a, [u8]>>,
    pub import_path2id: RkyvCowBytes<'a>,
    pub re_exports: Cow<'a, [u8]>,
}

/// The serialized form of a crate's documentation, as stored in the cache.
#[derive(Debug)]
pub struct CacheEntry<'a> {
    pub root_item_id: u32,
    pub external_crates: Cow<'a, [u8]>,
    pub paths: RkyvCowBytes<'a>,
    pub format_version: i64,
    pub items: RkyvCowBytes<'a>,
    pub secondary_indexes: Option<SecondaryIndexes<'a>>,
}

/// The key used to store and retrieve a crate's documentation from the cache.
///
/// It tries to capture all the information that can influence the output of the
/// relevant `rustdoc` command.
#[derive(Debug)]
pub struct ThirdPartyCrateCacheKey<'a> {
    pub crate_name: &'a str,
    pub crate_source: Cow<'a, str>,
    pub crate_version: String,
    /// The hash of the crate's source code.
    /// It is only populated for path dependencies.
    pub crate_hash: Option<String>,
    pub cargo_fingerprint: &'a str,
    pub rustdoc_options: String,
    pub default_feature_is_enabled: bool,
    pub active_named_features: String,
}
