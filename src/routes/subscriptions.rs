use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
// NOTE: `tracing::Instrument` is an extension trait for
// futures that makes spans interoperate with async code.
// Anytime a future is polled it enters the corresponding
// span, when the future is *parked* the span is exited.
use tracing::Instrument;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

// NOTE: The `web::Data` extractor is used to extract data
// from the application state. actix-web uses a *type-map*
// to represent its application-state: A `HashMap` that
// stores arbitrary data (using the `Any` type) using their
// unique type identifier as a key (`TypeId::off`). When a
// new request comes in, the `TypeId` of the type specified
// is computed and actix-web checks if the type-map contains
// the value specified in the handler.

// NOTE: This technique is similar to what other languages
// might call *dependency injection*!

pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    // NOTE: We *correlate* all logs (traces) related to the
    // same request using a *request* or *correlation id*.
    let request_id = Uuid::new_v4();
    // `tracing::info_span!` creates a span of log-level *info*,
    // however we still need to explicitly *step into* the span.
    // Once we do that, all subsequent spans/logs are considered
    // *children* of this span.

    // NOTE: You can enter/exit spans multiple times, this is handy
    // for asynchronous tasks for example. Closing is final on the
    // other hand.
    let request_span = tracing::info_span!(
        "adding a new subscriber",
        // The `tracing` create allows us to associate *structured
        // information* to spans as key-value pairs. A prefixed `%`
        // tells `tracing` to use their `Display` trait implementation.
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    // RAII pattern, the guard is dropped when it's scope ends.
    let _request_span_guard = request_span.enter();
    // This span will be *attached* to the future returned by
    // `sqlx::query!` which is made possible by the `Future`
    // extension trait `tracing::Instrument`.
    let query_span = tracing::info_span!("saving new subscriber details in the database",);
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    // We use `get_ref` to get an immutable ref to `PgPool`
    // which is wrapped by `web::Data`.
    .execute(pool.get_ref())
    // First we attach the instrumentation, then we `await` it.
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(err) => {
            // TODO: This log falls outside of `query_span` for now.
            tracing::error!("failed to execute query: {:?}", err);
            HttpResponse::InternalServerError()
        }
    }
}
