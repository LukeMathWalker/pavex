use api_server::configuration::{load_configuration, ApplicationProfile};
use api_server_sdk::{build_application_state, run};
use conduit_core::configuration::Config;
use pavex::hyper::Server;

pub struct TestApi {
    pub api_address: String,
    pub api_client: reqwest::Client,
}

impl TestApi {
    pub async fn spawn() -> Self {
        let config = Self::get_config();

        let application_state = build_application_state(&config.auth, &config.database)
            .await
            .expect("Failed to build the application state");

        let tcp_listener = config
            .server
            .listener()
            .expect("Failed to bind the server TCP listener");
        let address = tcp_listener
            .local_addr()
            .expect("The server TCP listener doesn't have a local socket address");
        let server_builder =
            Server::from_tcp(tcp_listener).expect("Failed to build a hyper Server");

        tokio::spawn(async move {
            run(server_builder, application_state)
                .await
                .expect("Failed to launch API server");
        });

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
