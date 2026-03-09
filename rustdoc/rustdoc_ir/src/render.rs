use std::fmt;
use std::fmt::Write;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use serde::{Deserializer, Serializer};

use crate::{Lifetime, Type};

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
    ///
    /// Named lifetimes are preserved as-is. Call sites that need `'_` should use
    /// [`Type::rename_lifetime_parameters`] beforehand to replace named lifetimes.
    pub fn syn_type(&self, id2name: &BiHashMap<PackageId, String>) -> syn::Type {
        let type_ = self.render_type(id2name);
        syn::parse_str(&type_).unwrap()
    }

    /// Render this type as a Rust source string, preserving named lifetimes as-is.
    pub fn render_type(&self, id2name: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        self.render_type_into(id2name, &mut buffer);
        buffer
    }

    pub fn render_type_into(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
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
                        argument.render_type_into(id2name, buffer);
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
                r.inner.render_type_into(id2name, buffer);
            }
            Type::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element.render_type_into(id2name, buffer);
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
                write!(buffer, "[").unwrap();
                s.element_type.render_type_into(id2name, buffer);
                write!(buffer, "]").unwrap();
            }
            Type::Array(a) => {
                write!(buffer, "[").unwrap();
                a.element_type.render_type_into(id2name, buffer);
                write!(buffer, "; {}]", a.len).unwrap();
            }
            Type::RawPointer(r) => {
                if r.is_mutable {
                    write!(buffer, "*mut ").unwrap();
                } else {
                    write!(buffer, "*const ").unwrap();
                }
                r.inner.render_type_into(id2name, buffer);
            }
            Type::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }

    /// Render this type as a Rust source string, replacing named lifetimes with `'_`.
    ///
    /// Use this when the type appears in a codegen expression context where the
    /// original named lifetimes are not in scope.
    pub fn render_with_inferred_lifetimes(&self, id2name: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        self.render_with_inferred_lifetimes_into(id2name, &mut buffer);
        buffer
    }

    pub fn render_with_inferred_lifetimes_into(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
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
                        argument.render_with_inferred_lifetimes_into(id2name, buffer);
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
                    Lifetime::Named(_) | Lifetime::Inferred => {
                        write!(buffer, "'_ ").unwrap();
                    }
                    Lifetime::Elided => {}
                }
                if r.is_mutable {
                    write!(buffer, "mut ").unwrap();
                }
                r.inner.render_with_inferred_lifetimes_into(id2name, buffer);
            }
            Type::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element.render_with_inferred_lifetimes_into(id2name, buffer);
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
                write!(buffer, "[").unwrap();
                s.element_type.render_with_inferred_lifetimes_into(id2name, buffer);
                write!(buffer, "]").unwrap();
            }
            Type::Array(a) => {
                write!(buffer, "[").unwrap();
                a.element_type.render_with_inferred_lifetimes_into(id2name, buffer);
                write!(buffer, "; {}]", a.len).unwrap();
            }
            Type::RawPointer(r) => {
                if r.is_mutable {
                    write!(buffer, "*mut ").unwrap();
                } else {
                    write!(buffer, "*const ").unwrap();
                }
                r.inner.render_with_inferred_lifetimes_into(id2name, buffer);
            }
            Type::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }

    /// Format this type for display in user-facing error messages.
    pub fn display_for_error(&self) -> String {
        let mut s = String::new();
        self.display_for_error_into(&mut s);
        s
    }

    pub fn display_for_error_into<W: fmt::Write>(&self, buffer: &mut W) {
        match self {
            Type::Path(t) => {
                write!(buffer, "{}", t.base_type.join("::")).unwrap();
                if !t.generic_arguments.is_empty() {
                    write!(buffer, "<").unwrap();
                    let mut arguments = t.generic_arguments.iter().peekable();
                    while let Some(argument) = arguments.next() {
                        argument.display_for_error_into(buffer);
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
                r.inner.display_for_error_into(buffer);
            }
            Type::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element.display_for_error_into(buffer);
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
                write!(buffer, "[").unwrap();
                s.element_type.display_for_error_into(buffer);
                write!(buffer, "]").unwrap();
            }
            Type::Array(a) => {
                write!(buffer, "[").unwrap();
                a.element_type.display_for_error_into(buffer);
                write!(buffer, "; {}]", a.len).unwrap();
            }
            Type::RawPointer(r) => {
                if r.is_mutable {
                    write!(buffer, "*mut ").unwrap();
                } else {
                    write!(buffer, "*const ").unwrap();
                }
                r.inner.display_for_error_into(buffer);
            }
            Type::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_for_error_into(f);
        Ok(())
    }
}
