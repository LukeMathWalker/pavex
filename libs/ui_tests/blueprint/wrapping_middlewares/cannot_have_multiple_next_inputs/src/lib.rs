use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::wrap]
pub fn mw<T>(_next: Next<T>, _second_next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
