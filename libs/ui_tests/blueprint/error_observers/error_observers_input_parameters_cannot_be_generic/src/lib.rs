use pavex::blueprint::Blueprint;
use pavex::Error;

pub struct GenericType<V>(V);

#[pavex::error_observer]
pub fn generic<T>(_generic_input: GenericType<T>, _e: &Error) {
    todo!()
}

#[pavex::error_observer]
pub fn generic2<T, S>(_i1: GenericType<T>, _i2: GenericType<S>, _e: &Error) {
    todo!()
}

#[pavex::error_observer]
pub fn generic3<T, S, U>(
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
    _e: &Error,
) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(GENERIC);
    bp.error_observer(GENERIC_2);
    bp.error_observer(GENERIC_3);
    bp
}
