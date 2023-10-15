//! An HTTP [`Server`] and its supporting types, the toolkit you need to launch your Pavex application.
//!
//! Check out [`Server`]'s documentation for more information.
pub use configuration::ServerConfiguration;
pub use incoming::IncomingStream;
pub use server::Server;
pub use server_handle::ServerHandle;
pub use shutdown_mode::ShutdownMode;

mod configuration;
mod incoming;
mod server;
mod server_handle;
mod shutdown_mode;
mod worker;
