use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{self, DatabaseSettings},
    telemetry,
};

// We use the `once_cell` crate to ensure that the `tracing` stack
// is initialized only once.
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    // We'll choose the tracing sink dynamically depending on the
    // value of the environment variable `TEST_LOG`. If set, we use
    // `std::io::stdout`, else we send all logs to `std::io::sink`.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        telemetry::init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spins up an instance of our application and returns its address
/// (i.e. `http://localhost:XXXX`).
async fn spawn_app() -> TestApp {
    // The first time `spawn_app` is invoked the code in `TRACING`
    // is executed. All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // Port 0 will trigger the OS to search for an available port.
    // We spawn the app using a random port so that multiple tests can
    // run in parallel without conflicting with each other.
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration =
        configuration::get_configuration().expect("failed to read configuration");
    // NOTE: In order to isolate all tests which interact with the database
    // we'll set up a database for each test. That's slower than wrapping
    // each test in an SQL transaction that's rolled back after the test
    // concluded, however it's way easier to implement.
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server =
        zero2prod::startup::run(listener, connection_pool.clone()).expect("failed to bind address");
    // Launch the server as a background task, else it would run
    // indefinitely, blocking us. Ignore the returned join handle.
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

// NOTE: We intentionally don't clean up the databases since we can easily
// just restart the postgres instance should performance start to deteriorate.

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("failed to connect to postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create database");

    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("failed to migrate the database");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(200, response.status().as_u16());

    // sqlx connects to Postgres at compile-time to check that queries
    // are well formed. Just like the sqlx-cli, it relies on the environment
    // variable `DATABASE_URL` for the connection string.
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!("ursula_le_guin@gmail.com", saved.email);
    assert_eq!("le guin", saved.name);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // This is an example of a *table-driven* (or *parameterised*) test.
    // Thorsten Ball made a lot of those when writing the Monkey programming
    // language.
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "the request didn't fail with `400 Bad Request` when the payload was {}.",
            error_message
        );
    }
}
