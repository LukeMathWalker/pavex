use pavex::request::body::errors::ExtractJsonBodyError;
use pavex::request::body::BufferedBody;

pub fn parse<T>(body: BufferedBody) -> JsonBody<T> {
    todo!()
}

pub fn fallible_parse<T>(body: BufferedBody) -> Result<JsonBody<T>, ExtractJsonBodyError> {
    todo!()
}

pub struct JsonBody<T>(T);
