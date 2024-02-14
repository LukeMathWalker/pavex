use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::request::RequestHead;

pub fn error() -> pavex::Error {
    todo!()
}

pub fn error_ref(request_head: &RequestHead) -> &pavex::Error {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.constructor(f!(crate::error), Lifecycle::RequestScoped);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.constructor(f!(crate::error_ref), Lifecycle::RequestScoped);
        bp
    });
    bp
}
