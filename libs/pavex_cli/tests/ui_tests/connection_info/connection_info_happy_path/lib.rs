use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::{connection::ConnectionInfo, response::Response};

pub fn get_connection_info(conn_info: &ConnectionInfo) -> Response {
    let peer_addr = conn_info.peer_addr();
    Response::ok().set_typed_body(format!("Success"))
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(crate::get_connection_info));
    bp
}
