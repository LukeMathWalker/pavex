use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn mw(_response: Response, _response2: Response) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn mw1(_response: Response, _response2: Response) -> Response {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(f!(crate::mw));
    bp.post_process(MW_1);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
