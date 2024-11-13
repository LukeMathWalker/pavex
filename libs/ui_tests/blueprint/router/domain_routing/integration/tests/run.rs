use std::future::IntoFuture;
use std::net::TcpListener;

use application::{build_application_state, run};

async fn spawn_test_server() -> Client {
    static TELEMETRY: std::sync::Once = std::sync::Once::new();
    TELEMETRY.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
            .init();
    });

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to listen on a random port");
    let port = listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    let incoming_stream: pavex::server::IncomingStream =
        listener.try_into().expect("Failed to convert listener");
    let server = pavex::server::Server::new().listen(incoming_stream);
    let application_state = build_application_state().await;
    tokio::task::spawn(run(server, application_state).into_future());
    Client::new(port)
}

struct Client {
    client: reqwest::Client,
    port: u16,
}

impl Client {
    fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            port,
        }
    }

    async fn get(&self, host: &str, path: &str) -> String {
        self.client
            .get(&format!("http://127.0.0.1:{}{path}", self.port))
            // We need to spoof the `Host` header to avoid having to set up
            // complicated local DNS resolution hacks.
            .header("Host", host)
            .send()
            .await
            .expect("Failed to make request")
            .text()
            .await
            .expect("Failed to get response body")
    }
}

#[tokio::test]
async fn routing() {
    let client = spawn_test_server().await;

    // `/` is available on all domains but one,
    // and it's routed correctly based on the `Host` header.
    assert_eq!(&client.get("company.com", "/").await, "company.com /");
    assert_eq!(&client.get("admin.company.com", "/").await, "admin.company.com /");
    assert_eq!(&client.get("ops.company.com", "/").await, "ops.company.com fallback");
    assert_eq!(&client.get("random.company.com", "/").await, "{sub}.company.com /");
    assert_eq!(&client.get("a.truly.random.company.com", "/").await, "{*any}.{sub}.company.com /");

    // `/login` is only available on `company.com`.
    // It goes to the respective fallbacks on other domains.
    assert_eq!(&client.get("company.com", "/login").await, "company.com /login");
    assert_eq!(&client.get("admin.company.com", "/login").await, "admin.company.com fallback");
    assert_eq!(&client.get("ops.company.com", "/login").await, "ops.company.com fallback");
    // The last two domains don't have a dedicated fallback, so they fall back to the root fallback.
    assert_eq!(&client.get("random.company.com", "/login").await, "root fallback [random.company.com]");
    assert_eq!(&client.get("a.truly.random.company.com", "/login").await, "root fallback [a.truly.random.company.com]");

    // If the domain is unknown, the root fallback is used.
    assert_eq!(&client.get("unknown.com", "/").await, "root fallback [unknown.com]");
    // The same happens if the `Host` header is not a valid domain...
    assert_eq!(&client.get("123", "/").await, "root fallback [123]");
    // or if it's missing. But `reqwest` makes it impossible to send a request without a `Host` header,
    // so we just reviewed the code to make sure it's correct.
}
