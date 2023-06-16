#[pavex_macros::RouteParams]
struct SimpleStruct {
    field1: i32,
    field2: String,
}

#[pavex_macros::RouteParams]
struct NestedStruct {
    field1: SimpleStruct,
}

#[pavex_macros::RouteParams]
struct StructWithOneGeneric<T> {
    field1: T,
    field2: String,
}

#[pavex_macros::RouteParams]
struct StructWithOneInlineBoundGeneric<T: std::fmt::Display> {
    field1: T,
    field2: String,
}

#[pavex_macros::RouteParams]
struct StructWithTwoGenerics<T, S> {
    field1: T,
    field2: S,
}

#[pavex_macros::RouteParams]
struct StructWithOneGenericAndALifetime<'a, S> {
    field1: &'a str,
    field2: S,
}

#[pavex_macros::RouteParams]
struct StructWithTwoLifetimes<'a, 'b: 'a> {
    field1: &'a str,
    field2: &'b str,
}

/// Verify that the given type implements the traits we expect.
fn has_required_traits<
    'a,
    T: pavex::serialization::StructuralDeserialize + serde::Deserialize<'a> + serde::Serialize,
>(
    _t: T,
) {
}

fn main() {
    has_required_traits(SimpleStruct {
        field1: 1,
        field2: "hello".to_string(),
    });
    has_required_traits(StructWithOneGeneric {
        field1: 1,
        field2: "hello".to_string(),
    });
    has_required_traits(StructWithOneInlineBoundGeneric {
        field1: 1,
        field2: "hello".to_string(),
    });
    has_required_traits(StructWithTwoGenerics {
        field1: 1,
        field2: "hello".to_string(),
    });
    has_required_traits(StructWithOneGenericAndALifetime {
        field1: "HEY",
        field2: "hello".to_string(),
    });
    has_required_traits(StructWithTwoLifetimes {
        field1: "HEY",
        field2: "hello",
    });
}
