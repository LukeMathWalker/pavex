#![allow(dead_code)]
#![allow(unused_variables)]

pub use blueprint::blueprint;

mod blueprint;
pub mod handler;
pub mod user;

pub use blueprint::{A, B};
