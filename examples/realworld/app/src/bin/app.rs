use app::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("realworld".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber)?;

    Ok(())
}
