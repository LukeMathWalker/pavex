use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::{linter::Lint, Blueprint};
use pavex::f;

pub fn register(bp: &mut Blueprint) {
    // How do you add your own configuration?
    // The starter template includes a "dummy" configuration option as
    // a reference example.
    //
    // Steps:
    //
    // 1. Define a public struct to group related configuration values together.
    //    E.g. `DatabaseConfig` for the host, port, username and password
    //    for a database connection.
    //    In this example, it's `DummyConfig`.
    // 2. Add the struct as a field to `ApplicationConfig`.
    //    You can see `pub dummy: DummyConfig` down there.
    // 3. Add an accessor method that returns a reference to the sub-field.
    //    In our example, [`AppConfig::dummy_config`].
    // 4. Add it as a constructor in `configuratio::register`, so that
    //    your components can inject it.
    // 5. (Optional) Add it to the configuration files stored in the
    //    `*_server/configuration` folder, or inject it at runtime via an
    //    environment variable.
    //    See `Config::load` for more details on how configuration is assembled.
    //
    // Feel free to delete `DummyConfig` once you get started working on your own app!
    bp.singleton(f!(self::AppConfig::dummy_config))
        .cloning(CloningStrategy::CloneIfNecessary)
        .ignore(Lint::Unused);
}

#[derive(serde::Deserialize, Debug, Clone)]
/// The configuration object holding all the values required
/// to configure the application.
pub struct AppConfig {
    pub dummy: DummyConfig,
}

impl AppConfig {
    pub fn dummy_config(&self) -> &DummyConfig {
        &self.dummy
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
/// A dummy group of configuration values to showcase how app config works.
/// Check out [`AppConfig::dummy`] for more details.
pub struct DummyConfig {
    pub flag: bool,
}
