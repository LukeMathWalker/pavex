use pavex_macros::get;

#[get] 
pub fn my_route() -> pavex::Response {
    pavex::Response::new(pavex::http::StatusCode::OK)
}

fn main() {
    let _response = my_route();
}