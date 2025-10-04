use pavex::Blueprint;

pub struct Logger;

#[pavex::transient]
pub fn new_logger() -> Logger {
    todo!()
}

pub fn add_logger(bp: &mut Blueprint) {
    bp.constructor(NEW_LOGGER);
}
