use pavex::blueprint::{from, Blueprint};
use pavex::{connection::ConnectionInfo, response::Response};

#[pavex::get(path = "/")]
pub fn get_connection_info(conn_info: &ConnectionInfo) -> Response {
    let _peer_addr = conn_info.peer_addr();
    Response::ok().set_typed_body("Success".to_string())
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
