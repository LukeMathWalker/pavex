use darling::FromMeta;
use errors::InvalidAttributeParams;

pub mod errors;
pub mod model;

/// Parse a raw `pavex` diagnostic attribute into the specification for a Pavex component.
///
/// It returns `None` for:
/// - attributes that don't belong to the `diagnostic::pavex` namespace (e.g. `#[inline]`)
/// - attributes that don't parse successfully into `syn::Attribute`
pub fn parse(attrs: &str) -> Result<Option<AnnotatedComponent>, errors::AttributeParserError> {
    let Ok(attrs) = parse_outer_attrs(attrs) else {
        return Ok(None);
    };
    let mut component = None;
    for attr in attrs {
        let Some(sub_path) = strip_pavex_path_prefix(attr.path()) else {
            return Ok(None);
        };
        let Some(component_kind) = sub_path.get_ident() else {
            return Err(errors::UnknownPavexAttribute {
                path: attr.path().to_owned(),
            }
            .into());
        };
        let c = match component_kind.to_string().as_str() {
            "constructor" => {
                let parsed = model::ConstructorProperties::from_meta(&attr.meta)
                    .map_err(InvalidAttributeParams::constructor)?;
                AnnotatedComponent::Constructor(parsed)
            }
            _ => {
                return Err(errors::UnknownPavexAttribute {
                    path: attr.path().to_owned(),
                }
                .into());
            }
        };
        if !component.is_none() {
            return Err(errors::AttributeParserError::MultiplePavexAttributes);
        } else {
            component = Some(c);
        }
    }
    Ok(component)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotatedComponent {
    Constructor(model::ConstructorProperties),
}

/// Strip the `diagnostic::pavex` prefix from a path.
///
/// It returns `None` if the path doesn't start with `diagnostic::pavex`.
/// It returns `Some` otherwise, yielding the remaining path segments.
fn strip_pavex_path_prefix(path: &syn::Path) -> Option<syn::Path> {
    if path.segments.len() < 2 {
        return None;
    }

    let prefix = &path.segments[0];
    let pavex = &path.segments[1];

    if prefix.ident == "diagnostic"
        && prefix.arguments.is_empty()
        && pavex.ident == "pavex"
        && pavex.arguments.is_empty()
    {
        let remaining_segments = path.segments.iter().skip(2).cloned().collect();
        let remaining_path = syn::Path {
            leading_colon: path.leading_colon,
            segments: remaining_segments,
        };
        Some(remaining_path)
    } else {
        None
    }
}

/// Extract outer attributes from a string of attributes.
/// I.e. attributes that start with `#[` rather than `#![`.
fn parse_outer_attrs(attrs: &str) -> syn::Result<Vec<syn::Attribute>> {
    /// `syn` doesn't let you parse outer attributes directly since `syn::Attribute` doesn't
    /// implement `syn::parse::Parse`.
    /// We use this thin wrapper to be able to invoke `syn::Attribute::parse_outer` on it.
    struct OuterAttributes(Vec<syn::Attribute>);

    impl syn::parse::Parse for OuterAttributes {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            input.call(syn::Attribute::parse_outer).map(OuterAttributes)
        }
    }
    syn::parse_str::<OuterAttributes>(attrs).map(|a| a.0)
}

#[cfg(test)]
mod tests {
    use model::{ConstructorProperties, Lifecycle};

    use super::*;

    #[test]
    fn test_inline() {
        assert_eq!(parse("#[inline]").unwrap(), None);
    }

    #[test]
    fn test_not_an_attribute() {
        assert_eq!(parse("inline").unwrap(), None);
    }

    #[test]
    fn test_unknown_pavex_attribute() {
        let err = parse("#[diagnostic::pavex::unknown]").unwrap_err();
        insta::assert_snapshot!(err, @"Unknown Pavex attribute: `#[diagnostic::pavex::unknown(...)]`");
    }

    #[test]
    fn test_invalid_constructor_lifecycle() {
        let err = parse(r#"#[diagnostic::pavex::constructor(lifecycle = "worker")]"#).unwrap_err();
        insta::assert_snapshot!(err, @"Unknown literal value `worker` at lifecycle for `pavex::diagnostic::constructor` attribute");
    }

    #[test]
    fn test_cloning_strategy_can_be_omitted() {
        let c = parse(r#"#[diagnostic::pavex::constructor(lifecycle = "singleton")]"#)
            .unwrap()
            .unwrap();
        assert_eq!(
            c,
            AnnotatedComponent::Constructor(ConstructorProperties {
                lifecycle: Lifecycle::Singleton,
                cloning_strategy: None,
                error_handler: None
            })
        );
    }

    #[test]
    fn test_unknown_property_for_constructor() {
        let err = parse(r#"#[diagnostic::pavex::constructor(lifecycle = "singleton", beautiful)]"#)
            .unwrap_err();
        insta::assert_snapshot!(err, @"Unknown field: `beautiful` for `pavex::diagnostic::constructor` attribute");
    }
}
