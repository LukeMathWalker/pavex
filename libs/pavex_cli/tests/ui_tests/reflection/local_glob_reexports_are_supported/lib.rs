use pavex_builder::{f, router::GET, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::nested::function));
    bp
}

pub mod nested {
    pub mod module {
        pub fn function() -> String {
            "Hello, world!".to_string()
        }
    }

    pub use module::*;
}