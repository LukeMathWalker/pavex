use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::response::Response;

pub mod clear;
pub mod client;
pub mod cycle_id;
pub mod delete;
pub mod get;
pub mod get_struct;
pub mod insert;
pub mod insert_struct;
pub mod invalidate;
pub mod remove;
pub mod remove_raw;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.route(GET, "/clear", f!(self::clear::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/client", f!(self::client::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/cycle_id", f!(self::cycle_id::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/delete", f!(self::delete::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/get", f!(self::get::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/get_struct", f!(self::get_struct::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/invalidate", f!(self::invalidate::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/remove", f!(self::remove::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/remove_raw", f!(self::remove_raw::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/insert", f!(self::insert::handler))
        .error_handler(f!(self::e500));
    bp.route(GET, "/insert_struct", f!(self::insert_struct::handler))
        .error_handler(f!(self::e500));
    bp
}

pub fn e500(_e: &pavex::Error) -> Response {
    Response::internal_server_error()
}
