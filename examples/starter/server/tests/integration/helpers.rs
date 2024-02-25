use pavex::server::Server;
use server::configuration::{ApplicationProfile, Config};
use server_sdk::{build_application_state, run};
use std::sync::Once;
use tracing::subscriber::set_global_default;
use tracing_subscriber::EnvFilter;

pub struct TestApi {
    pub api_address: String,
    pub api_client: reqwest::Client,
}

impl TestApi {
    pub async fn spawn() -> Self {
        Self::init_telemetry();
        let config = Self::get_config();

        let application_state = build_application_state().await;

        let tcp_listener = config
            .server
            .listener()
            .await
            .expect("Failed to bind the server TCP listener");
        let address = tcp_listener
            .local_addr()
            .expect("The server TCP listener doesn't have a local socket address");
        let server_builder = Server::new().listen(tcp_listener);

        tokio::spawn(async move { run(server_builder, application_state).await });

        TestApi {
            api_address: format!("http://{}:{}", config.server.ip, address.port()),
            api_client: reqwest::Client::new(),
        }
    }

    /// Load the dev configuration and tweak it to ensure that tests are
    /// properly isolated from each other.
    fn get_config() -> Config {
        let mut config =
            Config::load(Some(ApplicationProfile::Dev)).expect("Failed to load test configuration");
        // We use port `0` to get the operating system to assign us a random port.
        // This lets us run tests in parallel without running into "port X is already in use"
        // errors.
        config.server.port = 0;
        config
    }

    fn init_telemetry() {
        // Initialize the telemetry setup at most once.
        static INIT_TELEMETRY: Once = Once::new();
        INIT_TELEMETRY.call_once(|| {
            // Only enable the telemetry if the `TEST_LOG` environment variable is set.
            if std::env::var("TEST_LOG").is_ok() {
                let subscriber = tracing_subscriber::fmt::Subscriber::builder()
                    .with_env_filter(
                        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")),
                    )
                    .finish();
                // We don't redirect panic messages to the `tracing` subsystem because
                // we want to see them in the test output.
                set_global_default(subscriber).expect("Failed to set a `tracing` global subscriber")
            }
        });
    }
}

/// Convenient methods for calling the API under test.
impl TestApi {
    pub async fn get_ping(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/api/ping", &self.api_address))
            .send()
            .await
            .expect("Failed to execute request.")
    }
}
