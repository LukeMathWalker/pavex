use std::sync::LazyLock;

use reqwest::header::{HeaderValue, USER_AGENT};
use reqwest_middleware::{ClientWithMiddleware, Middleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use reqwest_tracing::TracingMiddleware;

pub static HTTP_CLIENT: LazyLock<ClientWithMiddleware> = LazyLock::new(http_client);

fn http_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(UserAgentInjector)
        .with(TracingMiddleware::default())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

/// Inject the CLI's user agent into all outgoing requests.
struct UserAgentInjector;

const CLI_USER_AGENT: HeaderValue =
    HeaderValue::from_static(concat!("pavex-cli/", env!("CARGO_PKG_VERSION")));

#[async_trait::async_trait]
impl Middleware for UserAgentInjector {
    async fn handle(
        &self,
        mut req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> Result<reqwest::Response, reqwest_middleware::Error> {
        req.headers_mut().insert(USER_AGENT, CLI_USER_AGENT);
        next.run(req, extensions).await
    }
}
