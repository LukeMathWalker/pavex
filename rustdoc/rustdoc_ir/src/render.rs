use std::fmt;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use serde::{Deserializer, Serializer};

use crate::function_pointer::write_fn_pointer_prefix;
use crate::{Lifetime, Type};

/// Configuration for rendering types and generic arguments.
///
/// The two dimensions—[`PathStyle`] and [`LifetimeStyle`]—capture the
/// differences between codegen rendering, inferred-lifetime rendering,
/// and error-message rendering, allowing a single traversal to serve
/// all three use cases.
#[derive(Clone, Copy)]
pub(crate) struct RenderConfig<'a> {
    /// How to render the crate-qualified portion of a path type.
    pub path: PathStyle<'a>,
    /// How to render named lifetime parameters.
    pub lifetime: LifetimeStyle,
}

/// Controls how the crate prefix of a [`Type::Path`] is rendered.
#[derive(Clone, Copy)]
pub(crate) enum PathStyle<'a> {
    /// Look up the crate name from a package-id-to-name mapping,
    /// then join the remaining segments with `::`.
    CrateLookup(&'a BiHashMap<PackageId, String>),
    /// Join all segments of `base_type` directly with `::`.
    Direct,
}

/// Controls how named lifetimes are rendered.
#[derive(Clone, Copy)]
pub(crate) enum LifetimeStyle {
    /// Preserve named lifetimes as-is (e.g. `'a` stays `'a`).
    Preserve,
    /// Replace named and inferred lifetimes with `'_`.
    Erase,
}

/// Serialize a [`PackageId`] as its string representation.
pub(crate) fn serialize_package_id<S>(
    package_id: &PackageId,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(package_id.repr())
}

/// Deserialize a [`PackageId`] from its string representation.
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

    /// Like [`Type::render_type`], but writes into an existing buffer instead of allocating.
    pub fn render_type_into(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Preserve,
        };
        self.render_into(&config, buffer);
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

    /// Like [`Type::render_with_inferred_lifetimes`], but writes into an existing buffer
    /// instead of allocating.
    pub fn render_with_inferred_lifetimes_into(
        &self,
        id2name: &BiHashMap<PackageId, String>,
        buffer: &mut String,
    ) {
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        self.render_into(&config, buffer);
    }

    /// Format this type for display in user-facing error messages.
    pub fn display_for_error(&self) -> String {
        let mut s = String::new();
        self.display_for_error_into(&mut s);
        s
    }

    /// Like [`Type::display_for_error`], but writes into an existing buffer instead of
    /// allocating.
    pub fn display_for_error_into<W: fmt::Write>(&self, buffer: &mut W) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        self.render_into(&config, buffer);
    }

    /// Render this type into `buffer` according to `config`.
    ///
    /// This is the single implementation behind [`Type::render_type_into`],
    /// [`Type::render_with_inferred_lifetimes_into`], and [`Type::display_for_error_into`].
    pub(crate) fn render_into<W: fmt::Write>(&self, config: &RenderConfig<'_>, buffer: &mut W) {
        match self {
            Type::Path(t) | Type::TypeAlias(t) => {
                match config.path {
                    PathStyle::CrateLookup(id2name) => {
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
                    }
                    PathStyle::Direct => {
                        write!(buffer, "{}", t.base_type.join("::")).unwrap();
                    }
                }
                if !t.generic_arguments.is_empty() {
                    write!(buffer, "<").unwrap();
                    let mut arguments = t.generic_arguments.iter().peekable();
                    while let Some(argument) = arguments.next() {
                        argument.render_into(config, buffer);
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
                    Lifetime::Named(l) => match config.lifetime {
                        LifetimeStyle::Preserve => {
                            write!(buffer, "'{} ", l.as_str()).unwrap();
                        }
                        LifetimeStyle::Erase => {
                            write!(buffer, "'_ ").unwrap();
                        }
                    },
                    Lifetime::Inferred => {
                        write!(buffer, "'_ ").unwrap();
                    }
                    Lifetime::Elided => {}
                }
                if r.is_mutable {
                    write!(buffer, "mut ").unwrap();
                }
                r.inner.render_into(config, buffer);
            }
            Type::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element.render_into(config, buffer);
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
                s.element_type.render_into(config, buffer);
                write!(buffer, "]").unwrap();
            }
            Type::Array(a) => {
                write!(buffer, "[").unwrap();
                a.element_type.render_into(config, buffer);
                write!(buffer, "; {}]", a.len).unwrap();
            }
            Type::RawPointer(r) => {
                if r.is_mutable {
                    write!(buffer, "*mut ").unwrap();
                } else {
                    write!(buffer, "*const ").unwrap();
                }
                r.inner.render_into(config, buffer);
            }
            Type::FunctionPointer(fp) => {
                write_fn_pointer_prefix(buffer, &fp.abi, fp.is_unsafe).unwrap();
                write!(buffer, "fn(").unwrap();
                let mut inputs = fp.inputs.iter().peekable();
                while let Some(input) = inputs.next() {
                    if let Some(name) = &input.name {
                        write!(buffer, "{name}: ").unwrap();
                    }
                    input.type_.render_into(config, buffer);
                    if inputs.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ")").unwrap();
                if let Some(output) = &fp.output {
                    write!(buffer, " -> ").unwrap();
                    output.render_into(config, buffer);
                }
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
