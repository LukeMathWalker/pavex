use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::nested::function));
    bp
}

pub mod nested {
    pub mod module {
        use pavex::http::StatusCode;
        pub fn function() -> StatusCode {
            StatusCode::OK
        }
    }

    pub use module::*;
}
