use pavex::blueprint::{router::GET, Blueprint};
use pavex::{f, t};

pub type MyTupleAlias = (bool, char, u8);
pub type MixedGenericsAlias<'a, T> = MixedGenerics<'a, T>;
pub type RemoteAlias = dep::IntermediateAlias;
pub type RemoteGenericAlias<T> = dep::IntermediateGenericAlias<T, T>;
pub type RemoteLifetimeAlias<'a> = dep::DoubleLifetimeType<'a, 'a>;

pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub struct MixedGenerics<'a, T> {
    _a: &'a T,
}

pub fn mixed_generics<'a, T>(_a: &'a T) -> MixedGenericsAlias<'a, T> {
    todo!()
}

pub fn handler_with_input_tuple<'a>(
    _input: MyTupleAlias,
    _a: &RemoteAlias,
    _b: &RemoteGenericAlias<bool>,
    _c: &RemoteLifetimeAlias<'a>,
    _d: MixedGenerics<'a, String>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(std::string::String));
    bp.request_scoped(f!(crate::RemoteLifetimeAlias::new));
    bp.request_scoped(f!(crate::mixed_generics));
    bp.singleton(f!(crate::constructor_with_output_tuple));
    bp.singleton(f!(crate::RemoteAlias::new));
    bp.singleton(f!(crate::RemoteGenericAlias::<std::primitive::bool>::new));
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
