use pavex::Response;

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

#[pavex::prebuilt]
pub use sqlx::PgPool;

#[pavex::error_handler]
pub fn e500(_e: &anyhow::Error) -> Response {
    Response::internal_server_error()
}
