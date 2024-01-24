#![allow(dead_code)]
#![allow(unused_variables)]

pub use blueprint::blueprint;
use pavex::request::path::PathParams;

pub mod base;
mod blueprint;
pub mod replace;
pub mod skip;
pub mod tweak;

pub fn custom_path_params<T>() -> PathParams<T> {
    todo!()
}
