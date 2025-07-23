use pavex::middleware::Next;
use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct Scoped;

impl Default for Scoped {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl Scoped {
    #[request_scoped(clone_if_necessary)]
    pub fn new() -> Scoped {
        todo!()
    }
}

#[pavex::wrap]
pub fn mw<C>(_s: Scoped, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::wrap]
pub fn mw2<C>(_s: &Scoped, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_s: Scoped) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(MW);
    bp.wrap(MW_2);
    bp.routes(from![crate]);
    bp
}
