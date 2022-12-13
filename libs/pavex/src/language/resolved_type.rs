use std::fmt::Formatter;
use std::fmt::Write;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use serde::{Deserializer, Serializer};

use crate::language::ImportPath;

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct ResolvedType {
    #[serde(serialize_with = "serialize_package_id")]
    #[serde(deserialize_with = "deserialize_package_id")]
    // `PackageId` does not implement serde::Deserialize/serde::Serialize, therefore we must
    // manually specify deserializer and serializer for make the whole `ResolvedType` (de)serializable.
    pub package_id: PackageId,
    /// The id associated with this type within the (JSON) docs for `package_id`.
    ///
    /// The id is optional to allow for flexible usage patterns - e.g. to leverage [`ResolveType`]
    /// to work with types that we want to code-generate into a new crate.  
    pub rustdoc_id: Option<rustdoc_types::Id>,
    pub base_type: ImportPath,
    pub generic_arguments: Vec<ResolvedType>,
    pub is_shared_reference: bool,
}

fn serialize_package_id<S>(package_id: &PackageId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(package_id.repr())
}

fn deserialize_package_id<'de, D>(deserializer: D) -> Result<PackageId, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(PackageId::new(s))
}

impl ResolvedType {
    pub fn syn_type(&self, id2name: &BiHashMap<&PackageId, String>) -> syn::Type {
        let type_ = self.render_type(id2name);
        syn::parse_str(&type_).unwrap()
    }

    pub fn render_type(&self, id2name: &BiHashMap<&PackageId, String>) -> String {
        let mut buffer = String::new();
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        let maybe_reference = if self.is_shared_reference { "&" } else { "" };
        write!(&mut buffer, "{}{}", maybe_reference, crate_name).unwrap();
        write!(&mut buffer, "::{}", self.base_type[1..].join("::")).unwrap();
        if !self.generic_arguments.is_empty() {
            write!(&mut buffer, "<").unwrap();
            let mut arguments = self.generic_arguments.iter().peekable();
            while let Some(argument) = arguments.next() {
                write!(&mut buffer, "{}", argument.render_type(id2name)).unwrap();
                if arguments.peek().is_some() {
                    write!(&mut buffer, ", ").unwrap();
                }
            }
            write!(&mut buffer, ">").unwrap();
        }
        buffer
    }
}

impl std::fmt::Debug for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let maybe_reference = if self.is_shared_reference { "&" } else { "" };
        write!(f, "{}{}", maybe_reference, self.base_type.join("::"))?;
        if !self.generic_arguments.is_empty() {
            write!(f, "<")?;
            let mut arguments = self.generic_arguments.iter().peekable();
            while let Some(argument) = arguments.next() {
                write!(f, "{:?}", argument)?;
                if arguments.peek().is_some() {
                    write!(f, ", ")?;
                }
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}
