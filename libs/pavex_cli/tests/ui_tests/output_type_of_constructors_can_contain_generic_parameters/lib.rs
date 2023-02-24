use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

pub fn json<T>() -> Json<T> {
    todo!()
}

pub struct Json<T>(T);

// The generic parameter of `Json` is fully specified in the handler's input type!
pub fn handler(json: Json<u8>) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::json), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
