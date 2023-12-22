use pavex::request::body::errors::ExtractJsonBodyError;
use pavex::request::body::BufferedBody;

pub fn parse<T>(body: BufferedBody) -> Json<T> {
    todo!()
}

pub fn fallible_parse<T>(body: BufferedBody) -> Result<Json<T>, ExtractJsonBodyError> {
    todo!()
}

pub struct Json<T>(T);
