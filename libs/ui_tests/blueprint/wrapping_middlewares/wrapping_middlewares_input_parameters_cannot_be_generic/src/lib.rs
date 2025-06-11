use pavex::blueprint::{from, Blueprint};
use pavex::middleware::Next;
use pavex::response::Response;

pub struct GenericType<V>(V);

#[pavex::wrap]
pub fn generic<A, T>(_next: Next<A>, _generic_input: GenericType<T>) -> Response
where
    A: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::wrap]
pub fn doubly_generic<A, T, S>(_next: Next<A>, _i1: GenericType<T>, _i2: GenericType<S>) -> Response
where
    A: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::wrap]
pub fn triply_generic<A, T, S, U>(
    _next: Next<A>,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Response
where
    A: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(GENERIC);
    bp.wrap(DOUBLY_GENERIC);
    bp.wrap(TRIPLY_GENERIC);
    bp.routes(from![crate]);
    bp
}
