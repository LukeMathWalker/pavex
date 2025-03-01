use pavex::blueprint::Blueprint;
use pavex::t;

pub fn register(bp: &mut Blueprint) {
    // How do you add your own configuration?
    // The starter template includes `GreetConfig` as a reference example.
    //
    // Steps:
    //
    // 1. Define a public struct to group related configuration values together.
    //    E.g. `DatabaseConfig` for the host, port, username and password
    //    for a database connection.
    //    In this example, it's `GreetConfig`.
    // 2. Register the struct as configuration with the blueprint via
    //    `bp.config`, as you can see below.
    // 3. (Optional) Add it to the configuration files stored in the
    //    `*_server/configuration` folder, or inject it at runtime via an
    //    environment variable.
    //    See `Config::load` for more details on how configuration is assembled.
    //
    // Feel free to delete `GreetConfig` once you get started working on your own app!
    bp.config("greet", t!(self::GreetConfig));
}

#[derive(serde::Deserialize, Clone, Debug)]
/// A group of configuration values to showcase how app config works.
/// Check out [`register`]'s docs for more details.
pub struct GreetConfig {
    /// Say "Hello {name}," rather than "Hello," in the response.
    pub use_name: bool,
    /// The message that's appended after the "Hello" line.
    pub greeting_message: String,
}
