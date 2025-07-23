use pavex::middleware::Next;
use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct Singleton;

impl Default for Singleton {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl Singleton {
    #[singleton(clone_if_necessary)]
    pub fn new() -> Singleton {
        todo!()
    }
}

#[pavex::wrap]
pub fn mw<C>(_s: Singleton, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_s: Singleton) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
