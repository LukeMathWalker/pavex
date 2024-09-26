use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;

pub struct Logger;

pub fn new_logger() -> Logger {
    todo!()
}

pub fn add_logger(bp: &mut Blueprint) {
    bp.constructor(f!(crate::new_logger), Lifecycle::Transient);
}
