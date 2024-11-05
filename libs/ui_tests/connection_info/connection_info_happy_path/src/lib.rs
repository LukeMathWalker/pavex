use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::{connection::ConnectionInfo, response::Response};

pub fn get_connection_info(conn_info: &ConnectionInfo) -> Response {
    let _peer_addr = conn_info.peer_addr();
    Response::ok().set_typed_body("Success".to_string())
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(crate::get_connection_info));
    bp
}
