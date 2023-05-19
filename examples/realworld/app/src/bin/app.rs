use app::telemetry::{get_subscriber, init_telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("realworld".into(), "info".into(), std::io::stdout);
    init_telemetry(subscriber)?;

    let _configuration = app::configuration::load_configuration()?;

    Ok(())
}
