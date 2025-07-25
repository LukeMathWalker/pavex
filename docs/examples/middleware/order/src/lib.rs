#![allow(dead_code)]
#![allow(unused_variables)]
pub use blueprint::blueprint;

mod blueprint;
pub mod core;
mod mw;
pub mod order1;
pub mod order2;
pub mod post_and_wrap;
pub mod post_only;
pub mod pre_and_post;
pub mod pre_and_wrap;
pub mod pre_only;
pub mod routes;
pub mod wrap_only;
pub use mw::*;

pub use routes::GET_INDEX;
