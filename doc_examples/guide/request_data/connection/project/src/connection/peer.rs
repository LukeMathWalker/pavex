use pavex::connection::ConnectionInfo;
use pavex::response::Response;

pub fn handler(conn: ConnectionInfo) -> Response {
    let addr = conn.peer_addr().to_string();
    Response::ok().set_typed_body(format!("Your address is {addr}"))
}
