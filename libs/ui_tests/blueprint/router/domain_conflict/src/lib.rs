use pavex::Blueprint;
use pavex::Response;

#[pavex::get(path = "/")]
pub fn sub_root() -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn any_root() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{*any}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(ANY_ROOT);
        bp
    });
    bp.domain("{sub}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(SUB_ROOT);
        bp
    });
    bp
}
