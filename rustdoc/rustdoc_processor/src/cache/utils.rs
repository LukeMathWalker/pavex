//! Utility types for the cache module.

use rkyv::util::AlignedVec;
use rusqlite::{ToSql, types::ToSqlOutput};

/// A `Cow` variant to work with `rkyv`'s `AlignedVec`.
#[derive(Debug)]
pub enum RkyvCowBytes<'a> {
    Borrowed(&'a [u8]),
    Owned(AlignedVec),
}

impl ToSql for RkyvCowBytes<'_> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = match self {
            RkyvCowBytes::Borrowed(items) => items,
            RkyvCowBytes::Owned(s) => s.as_slice(),
        };
        Ok(ToSqlOutput::Borrowed(rusqlite::types::ValueRef::Blob(s)))
    }
}

impl RkyvCowBytes<'_> {
    pub fn into_owned(self) -> AlignedVec {
        match self {
            RkyvCowBytes::Borrowed(items) => {
                let mut v = AlignedVec::with_capacity(items.len());
                v.extend_from_slice(items);
                v
            }
            RkyvCowBytes::Owned(aligned_vec) => aligned_vec,
        }
    }
}

impl AsRef<[u8]> for RkyvCowBytes<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            RkyvCowBytes::Borrowed(items) => items,
            RkyvCowBytes::Owned(aligned_vec) => aligned_vec.as_slice(),
        }
    }
}
