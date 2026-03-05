use std::fmt::Write;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use serde::{Deserializer, Serializer};

use crate::{GenericArgument, Lifetime, Type};

pub(crate) fn serialize_package_id<S>(package_id: &PackageId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(package_id.repr())
}

pub(crate) fn deserialize_package_id<'de, D>(deserializer: D) -> Result<PackageId, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(PackageId::new(s))
}

impl Type {
    /// Parse this type into a [`syn::Type`], using the provided package-id-to-crate-name mapping.
    pub fn syn_type(&self, id2name: &BiHashMap<PackageId, String>) -> syn::Type {
        let type_ = self.render_type(id2name);
        syn::parse_str(&type_).unwrap()
    }

    /// Render this type as a Rust source string, using the provided package-id-to-crate-name mapping.
    pub fn render_type(&self, id2name: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        self._render_type(id2name, &mut buffer);
        buffer
    }

    fn _render_type(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            Type::Path(t) => {
                let crate_name = id2name
                    .get_by_left(&t.package_id)
                    .with_context(|| {
                        format!(
                            "The package id '{}' is missing from the id<>name mapping for crates.",
                            t.package_id
                        )
                    })
                    .unwrap();
                write!(buffer, "{crate_name}").unwrap();
                write!(buffer, "::{}", t.base_type[1..].join("::")).unwrap();
                if !t.generic_arguments.is_empty() {
                    write!(buffer, "<").unwrap();
                    let mut arguments = t.generic_arguments.iter().peekable();
                    while let Some(argument) = arguments.next() {
                        match argument {
                            GenericArgument::TypeParameter(t) => {
                                write!(buffer, "{}", t.render_type(id2name)).unwrap();
                            }
                            GenericArgument::Lifetime(l) => {
                                write!(buffer, "{l}").unwrap();
                            }
                        }
                        if arguments.peek().is_some() {
                            write!(buffer, ", ").unwrap();
                        }
                    }
                    write!(buffer, ">").unwrap();
                }
            }
            Type::Reference(r) => {
                write!(buffer, "&").unwrap();
                match &r.lifetime {
                    Lifetime::Static => {
                        write!(buffer, "'static ").unwrap();
                    }
                    Lifetime::Named(l) => {
                        write!(buffer, "'{} ", l.as_str()).unwrap();
                    }
                    Lifetime::Inferred => {
                        write!(buffer, "'_ ").unwrap();
                    }
                    Lifetime::Elided => {}
                }
                if r.is_mutable {
                    write!(buffer, "mut ").unwrap();
                }
                r.inner._render_type(id2name, buffer);
            }
            Type::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element._render_type(id2name, buffer);
                    if elements.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ")").unwrap();
            }
            Type::ScalarPrimitive(s) => {
                write!(buffer, "{s}").unwrap();
            }
            Type::Slice(s) => {
                write!(buffer, "[{}]", s.element_type.render_type(id2name)).unwrap();
            }
            Type::Array(a) => {
                write!(buffer, "[{}; {}]", a.element_type.render_type(id2name), a.len).unwrap();
            }
            Type::RawPointer(r) => {
                if r.is_mutable {
                    write!(buffer, "*mut ").unwrap();
                } else {
                    write!(buffer, "*const ").unwrap();
                }
                r.inner._render_type(id2name, buffer);
            }
            Type::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }
}
