use anyhow::Context;
use pavex::config::ConfigLoader;
use pavex::server::{Server, ServerHandle, ShutdownMode};
use server::{
    configuration::Profile,
    telemetry::{get_subscriber, init_telemetry},
};
use server_sdk::{ApplicationConfig, ApplicationState, run};
use std::time::Duration;
use tracing_log_error::log_error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from a .env file, if it exists.
    let _ = dotenvy::dotenv();

    let subscriber = get_subscriber("demo".into(), "info".into(), std::io::stdout);
    init_telemetry(subscriber)?;

    // We isolate all the server setup and launch logic in a separate function
    // to have a single point for logging fatal errors that cause the application to exit.
    if let Err(e) = _main().await {
        log_error!(*e, "The application is exiting due to an error");
    }

    Ok(())
}

async fn _main() -> anyhow::Result<()> {
    let config: ApplicationConfig = ConfigLoader::<Profile>::new().load()?;
    let tcp_listener = config
        .server
        .listener()
        .await
        .context("Failed to bind the server TCP listener")?;
    let address = tcp_listener
        .local_addr()
        .context("The server TCP listener doesn't have a local socket address")?;
    let server_builder = Server::new().listen(tcp_listener);
    let shutdown_timeout = config.server.graceful_shutdown_timeout;

    let application_state = ApplicationState::new(config)
        .await
        .context("Failed to build the application state")?;

    tracing::info!("Starting to listen for incoming requests at {}", address);
    let server_handle = run(server_builder, application_state);
    graceful_shutdown(server_handle.clone(), shutdown_timeout).await;
    server_handle.await;
    Ok(())
}

async fn graceful_shutdown(server_handle: ServerHandle, timeout: Duration) {
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for the Ctrl+C signal");
        server_handle
            .shutdown(ShutdownMode::Graceful { timeout })
            .await;
    });
}
