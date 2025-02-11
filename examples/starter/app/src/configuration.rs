use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::t;

pub fn register(bp: &mut Blueprint) {
    bp.prebuilt(t!(self::AppConfig));
    // How do you add your own configuration?
    // The starter template includes `GreetConfig` as a reference example.
    //
    // Steps:
    //
    // 1. Define a public struct to group related configuration values together.
    //    E.g. `DatabaseConfig` for the host, port, username and password
    //    for a database connection.
    //    In this example, it's `GreetConfig`.
    // 2. Add the struct as a field to `ApplicationConfig`.
    //    You can see `pub greet: GreetConfig` down there.
    // 3. Add an accessor method that returns a reference to the sub-field.
    //    In our example, [`AppConfig::greet_config`].
    // 4. Add it as a constructor in `configuratio::register`, so that
    //    your components can inject it.
    // 5. (Optional) Add it to the configuration files stored in the
    //    `*_server/configuration` folder, or inject it at runtime via an
    //    environment variable.
    //    See `Config::load` for more details on how configuration is assembled.
    //
    // Feel free to delete `DummyConfig` once you get started working on your own app!
    bp.transient(f!(self::AppConfig::greet_config));
}

#[derive(serde::Deserialize, Debug, Clone)]
/// The configuration object holding all the values required
/// to configure the application.
pub struct AppConfig {
    pub greet: GreetConfig,
}

impl AppConfig {
    pub fn greet_config(&self) -> &GreetConfig {
        &self.greet
    }
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
