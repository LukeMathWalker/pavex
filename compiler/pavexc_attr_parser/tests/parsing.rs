use pavex_bp_schema::Lifecycle;
use pavexc_attr_parser::{AnnotationProperties, errors};

// Convenience function to parse a single attribute string.
fn parse(attrs: &str) -> Result<Option<AnnotationProperties>, errors::AttributeParserError> {
    pavexc_attr_parser::parse(std::iter::once(attrs))
}

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
    let err =
        parse(r#"#[diagnostic::pavex::constructor(id = "B", lifecycle = "worker")]"#).unwrap_err();
    insta::assert_snapshot!(err, @"Unknown value: `worker`. Available values: `request_scoped`, `singleton`, `transient` at lifecycle for `pavex::diagnostic::constructor` attribute");
}

#[test]
fn test_cloning_policy_can_be_omitted() {
    let c = parse(r#"#[diagnostic::pavex::constructor(id = "B", lifecycle = "singleton")]"#)
        .unwrap()
        .unwrap();
    assert_eq!(
        c,
        AnnotationProperties::Constructor {
            id: "B".into(),
            lifecycle: Lifecycle::Singleton,
            cloning_policy: None,
            allow_unused: None,
            allow_error_fallback: None,
        }
    );
}

#[test]
fn test_unknown_property_for_constructor() {
    let err =
        parse(r#"#[diagnostic::pavex::constructor(id = "B", lifecycle = "singleton", beautiful)]"#)
            .unwrap_err();
    insta::assert_snapshot!(err, @"Unknown field: `beautiful`. Available values: `allow_error_fallback`, `allow_unused`, `cloning_policy`, `id`, `lifecycle` for `pavex::diagnostic::constructor` attribute");
}
