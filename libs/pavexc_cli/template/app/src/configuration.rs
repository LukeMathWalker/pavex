use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::Blueprint;
use pavex::f;

#[derive(serde::Deserialize, Debug, Clone)]
/// The configuration object holding all the values required
/// to configure the application.
pub struct AppConfig {
    /// How should go about adding your own configuration?
    /// We'll use a "dummy" configuration value to showcase how you'd go about it!
    ///
    /// Steps:
    ///
    /// 1. Define a public struct to group related configuration values together.
    ///    E.g. `DatabaseConfig` for the host, port, username and password
    ///    for a database connection.
    ///    In this example, it's `DummyConfig`.
    /// 2. Add the struct as a field to `ApplicationConfig`.
    ///    You can see `pub dummy: DummyConfig` here.
    /// 3. Add an accessor method that returns a reference to the sub-field.
    ///    In our example, [`AppConfig::dummy_config`].
    /// 4. Add it as a constructor in `ApplicationConfig::register`, so that
    ///    your components can inject it.
    /// 5. (Optional) Add it to the configuration files stored in the
    ///    `*_server/configuration` folder, or inject it at runtime via an
    ///    environment variable.
    ///    See `Config::load` for more details on how configuration is assembled.
    ///
    /// Feel free to delete `DummyConfig` once you get started working on your own app!
    pub dummy: DummyConfig,
}

impl AppConfig {
    pub fn dummy_config(&self) -> &DummyConfig {
        &self.dummy
    }

    pub fn register(bp: &mut Blueprint) {
        bp.singleton(f!(self::AppConfig::dummy_config))
            .cloning(CloningStrategy::CloneIfNecessary);
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
/// A dummy group of configuration values to showcase how app config works.
/// Check out [`AppConfig::dummy`] for more details.
pub struct DummyConfig {
    pub flag: bool,
}
