use pavex::{Response, get, request::path::PathParams};

use crate::user_agent::UserAgent;

#[PathParams]
pub struct GreetParams {
    pub name: String,
}

#[get(path = "/greet/{name}")]
pub fn greet(params: PathParams<GreetParams>, user_agent: UserAgent) -> Response {
    if let UserAgent::Unknown = user_agent {
        let body = "You must provide a `User-Agent` header";
        return Response::unauthorized().set_typed_body(body);
    }

    let GreetParams { name } = params.0;
    Response::ok().set_typed_body(format!("Hello, {name}!"))
}
