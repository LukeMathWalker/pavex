use pavex::blueprint::Blueprint;
use pavex::blueprint::from;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate::nested]);
    bp
}

pub mod nested {
    pub mod module {
        use pavex::http::StatusCode;
        
        #[pavex::get(path = "/home")]
        pub fn function() -> StatusCode {
            StatusCode::OK
        }
    }

    pub use module::*;
}
