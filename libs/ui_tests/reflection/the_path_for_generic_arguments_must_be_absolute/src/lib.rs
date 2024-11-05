use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub struct Logger<T>(T);

pub fn new_logger<T>() -> Logger<T> {
    todo!()
}

pub fn handler<T>(_logger: Logger<T>) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::new_logger::<String>));
    bp.route(GET, "/home", f!(crate::handler::<std::string::String>));
    bp
}
