use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::{connection::ConnectionInfo, response::Response};

pub fn root() -> Response {
    Response::ok()
}

#[pavex::fallback]
pub fn get_connection_info(conn_info: &ConnectionInfo) -> Response {
    let peer_addr = conn_info.peer_addr();
    Response::ok().set_typed_body(format!("{peer_addr}"))
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/route", f!(crate::root));
    bp.fallback(GET_CONNECTION_INFO);
    bp
}
