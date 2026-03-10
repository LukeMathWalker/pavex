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

impl<'a> CacheEntry<'a> {
    /// Create a cache entry from a crate, including secondary indexes.
    pub fn from_crate<A: serde::Serialize>(
        krate: &'a crate::queries::Crate,
        annotations: &'a A,
    ) -> Result<CacheEntry<'a>, anyhow::Error> {
        use rkyv::rancor::Panic;
        use rkyv::util::AlignedVec;

        let external_crates = bincode::serde::encode_to_vec(
            &krate.core.krate.external_crates,
            bincode::config::standard(),
        )?;

        let paths: AlignedVec = match &krate.core.krate.paths {
            crate::crate_data::CrateItemPaths::Eager(crate::crate_data::EagerCrateItemPaths {
                paths,
            }) => rkyv::to_bytes::<Panic>(paths)?,
            crate::crate_data::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        let items: AlignedVec = match &krate.core.krate.index {
            crate::crate_data::CrateItemIndex::Eager(crate::crate_data::EagerCrateItemIndex {
                index,
            }) => rkyv::to_bytes::<Panic>(index)?,
            crate::crate_data::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
        };

        let import_path2id: AlignedVec = match &krate.import_path2id {
            crate::indexing::ImportPath2Id::Eager(crate::indexing::EagerImportPath2Id(m)) => {
                rkyv::to_bytes::<Panic>(m)?
            }
            crate::indexing::ImportPath2Id::Lazy(lazy) => lazy.0.clone(),
        };

        let import_index =
            bincode::serde::encode_to_vec(&krate.import_index, bincode::config::standard())?;
        let annotated_items =
            bincode::serde::encode_to_vec(annotations, bincode::config::standard())?;
        let re_exports =
            bincode::serde::encode_to_vec(&krate.external_re_exports, bincode::config::standard())?;

        let secondary_indexes = SecondaryIndexes {
            import_index: Cow::Owned(import_index),
            annotated_items: Some(Cow::Owned(annotated_items)),
            import_path2id: RkyvCowBytes::Owned(import_path2id),
            re_exports: Cow::Owned(re_exports),
        };

        Ok(CacheEntry {
            root_item_id: krate.core.krate.root_item_id.0,
            external_crates: Cow::Owned(external_crates),
            paths: RkyvCowBytes::Owned(paths),
            format_version: krate.core.krate.format_version as i64,
            items: RkyvCowBytes::Owned(items),
            secondary_indexes: Some(secondary_indexes),
        })
    }

    /// Create a raw cache entry from a crate (no secondary indexes).
    pub fn from_crate_raw(
        krate: &'a crate::queries::Crate,
    ) -> Result<CacheEntry<'a>, anyhow::Error> {
        use rkyv::rancor::Panic;
        use rkyv::util::AlignedVec;

        let external_crates = bincode::serde::encode_to_vec(
            &krate.core.krate.external_crates,
            bincode::config::standard(),
        )?;

        let paths: AlignedVec = match &krate.core.krate.paths {
            crate::crate_data::CrateItemPaths::Eager(crate::crate_data::EagerCrateItemPaths {
                paths,
            }) => rkyv::to_bytes::<Panic>(paths)?,
            crate::crate_data::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        let items: AlignedVec = match &krate.core.krate.index {
            crate::crate_data::CrateItemIndex::Eager(crate::crate_data::EagerCrateItemIndex {
                index,
            }) => rkyv::to_bytes::<Panic>(index)?,
            crate::crate_data::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
        };

        Ok(CacheEntry {
            root_item_id: krate.core.krate.root_item_id.0,
            external_crates: Cow::Owned(external_crates),
            paths: RkyvCowBytes::Owned(paths),
            format_version: krate.core.krate.format_version as i64,
            items: RkyvCowBytes::Owned(items),
            secondary_indexes: None,
        })
    }
}

/// The key used to store and retrieve a crate's documentation from the cache.
///
/// It tries to capture all the information that can influence the output of the
/// relevant `rustdoc` command.
#[derive(Debug)]
pub(crate) struct ThirdPartyCrateCacheKey<'a> {
    pub(crate) crate_name: &'a str,
    pub(crate) crate_source: Cow<'a, str>,
    pub(crate) crate_version: String,
    /// The hash of the crate's source code.
    /// It is only populated for path dependencies.
    pub(crate) crate_hash: Option<String>,
    pub(crate) cargo_fingerprint: &'a str,
    pub(crate) rustdoc_options: String,
    pub(crate) default_feature_is_enabled: bool,
    pub(crate) active_named_features: String,
}
