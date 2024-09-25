use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::Error;

pub struct GenericType<V>(V);

pub fn generic<T>(generic_input: GenericType<T>, e: &Error) {
    todo!()
}

pub fn generic2<T, S>(i1: GenericType<T>, i2: GenericType<S>, e: &Error) {
    todo!()
}

pub fn generic3<T, S, U>(i1: GenericType<T>, i2: GenericType<S>, i3: GenericType<U>, e: &Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(f!(crate::generic));
    bp.error_observer(f!(crate::generic2));
    bp.error_observer(f!(crate::generic3));
    bp
}
