use pavex::blueprint::{
    from,
    router::{ANY, GET},
    Blueprint,
};
use pavex::f;

pub fn handler_1() -> pavex::response::Response {
    todo!()
}

pub fn handler_2() -> pavex::response::Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler_ann_1() -> pavex::response::Response {
    todo!()
}

#[pavex::route(path = "/", allow(any_method))]
pub fn handler_ann_2() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp.prefix("/bp").nest({
        let mut bp = Blueprint::new();
        bp.route(ANY, "/", f!(crate::handler_1));
        bp.route(GET, "/", f!(crate::handler_2));
        bp
    });
    bp
}
