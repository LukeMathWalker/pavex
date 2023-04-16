use pavex_builder::{
    f,
    router::{ANY, GET},
    Blueprint,
};

pub fn handler_1() -> pavex_runtime::response::Response {
    todo!()
}

pub fn handler_2() -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(ANY, "/home", f!(crate::handler_1));
    bp.route(GET, "/home", f!(crate::handler_2));
    bp
}
