use pavex_builder::{f, router::GET, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::nested::function));
    bp
}

pub mod nested {
    pub mod module {
        use pavex_runtime::http::StatusCode;
        pub fn function() -> StatusCode {
            StatusCode::OK
        }
    }

    pub use module::*;
}