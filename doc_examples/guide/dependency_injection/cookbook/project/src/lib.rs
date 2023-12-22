#![allow(dead_code)]

pub use blueprint::blueprint;

mod blueprint;
pub mod functions;
pub mod input;
pub mod non_static_methods;
pub mod output;
pub mod routes;
pub mod static_methods;
pub mod trait_methods;

pub struct User;
