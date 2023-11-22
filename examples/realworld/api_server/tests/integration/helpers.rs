use api_server::configuration::{load_configuration, ApplicationProfile};
use api_server_sdk::{build_application_state, run};
use conduit_core::configuration::Config;
use pavex::server::Server;
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

        let application_state = build_application_state(&config.auth, &config.database)
            .await
            .expect("Failed to build the application state");

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

    fn get_config() -> Config {
        let mut config = load_configuration(Some(ApplicationProfile::Test))
            .expect("Failed to load test configuration");

        // We generate the key pair on the fly rather than hardcoding it in the
        // configuration file.
        let key_pair = jwt_simple::algorithms::Ed25519KeyPair::generate();
        config.auth.eddsa_public_key_pem = key_pair.public_key().to_pem();
        config.auth.eddsa_private_key_pem = secrecy::Secret::new(key_pair.to_pem());
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
    pub async fn post_signup<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/users", &self.api_address))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}
