use pavex::{blueprint::from, Blueprint};

#[pavex::prebuilt]
pub use std::string::String;

pub type MyTupleAlias = (bool, char, u8);
pub type MixedGenericsAlias<'a, T> = MixedGenerics<'a, T>;
pub type RemoteAlias = dep::IntermediateAlias;
pub type RemoteGenericAlias<T> = dep::IntermediateGenericAlias<T, T>;
#[pavex::prebuilt]
pub type RemoteAssignedGenericAlias = dep::IntermediateGenericAlias<u8, u8>;
pub type RemoteLifetimeAlias<'a> = dep::DoubleLifetimeType<'a, 'a>;
#[pavex::prebuilt]
pub use dep::IntermediateAssignedGenericAlias;

#[pavex::singleton]
pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub struct MixedGenerics<'a, T> {
    _a: &'a T,
}

#[pavex::request_scoped]
pub fn mixed_generics<T>(_a: &T) -> MixedGenericsAlias<'_, T> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler_with_input_tuple<'a>(
    _input: MyTupleAlias,
    _a: &RemoteAlias,
    _b: &RemoteGenericAlias<bool>,
    _c: &RemoteLifetimeAlias<'a>,
    _d: MixedGenerics<'a, String>,
    // Type aliases are resolved and Pavex can see they desugar to the same type.
    _e: &RemoteGenericAlias<u8>,
    _f: &dep::GenericType<bool, u8>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, dep]);
    bp.routes(from![crate]);
    bp
}
