use env_logger::Env;
use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::{configuration, startup};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // `env_logger` filters the log levels from the environment
    // variable `RUST_LOG`. If it isn't set we fall back to logs
    // of the level *info* or above.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = configuration::get_configuration().expect("failed to read configuration");
    let pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("failed to connect to postgres");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    startup::run(listener, pool)?.await
}
