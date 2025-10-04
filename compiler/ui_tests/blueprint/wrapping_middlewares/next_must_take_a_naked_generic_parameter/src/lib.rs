use pavex::middleware::Next;
use pavex::Response;
use pavex::wrap;
use pavex::{blueprint::from, Blueprint};
use std::future::Ready;

pub struct Custom<T>(T);

impl<T> IntoFuture for Custom<T> {
    type Output = Response;
    type IntoFuture = Ready<Response>;

    fn into_future(self) -> Self::IntoFuture {
        todo!()
    }
}

#[wrap]
pub fn mw<T>(_next: Next<Custom<T>>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
