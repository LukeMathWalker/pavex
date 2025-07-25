use crate::configuration::GreetConfig;
use pavex::Response;
use pavex::get;
use pavex::request::path::PathParams;

#[PathParams]
pub struct GreetParams {
    name: String,
}

/// Response with a preconfigured message, greeting the caller.
#[get(path = "/greet/{name}")]
pub fn greet(params: PathParams<GreetParams>, config: &GreetConfig) -> Response {
    let body = if config.use_name {
        format!("Hello {},\n{}", params.0.name, config.greeting_message)
    } else {
        format!("Hello,\n{}", config.greeting_message)
    };
    Response::ok().set_typed_body(body)
}
