use pavexc_attr_parser::{AnnotationProperties, errors::AttributeParserError};
use rustdoc_types::Attribute;

/// Parse Pavex's attributes out of the attributes attached to a Rust item.
pub fn parse_pavex_attributes(
    attrs: &[Attribute],
) -> Result<Option<AnnotationProperties>, AttributeParserError> {
    // Attributes from the diagnostic namespace are exposed as-is by `rustdoc-json`.
    let relevant_attrs = attrs.iter().filter_map(|a| {
        if let Attribute::Other(a) = a {
            Some(a.as_str())
        } else {
            None
        }
    });
    pavexc_attr_parser::parse(relevant_attrs)
}
