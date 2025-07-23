use crate::pool::DbConnectionPool;
use pavex::get;
use pavex::http::StatusCode;

#[get(path = "/")]
pub fn get_index(_a: &DbConnectionPool) -> StatusCode {
    todo!()
}
