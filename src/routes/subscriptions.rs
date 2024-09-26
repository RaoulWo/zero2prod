use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

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
    // NOTE: We *correlate* all logs related to the same request
    // using a *request* or *correlation id*.
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {} - adding '{}' '{}' as a new subscriber",
        request_id,
        form.email,
        form.name
    );
    log::info!(
        "request_id {} - saving new subscriber details in the database",
        request_id
    );
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
    .await
    {
        Ok(_) => {
            log::info!(
                "request_id {} - new subscriber details have been saved",
                request_id
            );
            HttpResponse::Ok()
        }
        Err(err) => {
            log::error!(
                "request_id {} - failed to execute query: {:?}",
                request_id,
                err
            );
            HttpResponse::InternalServerError()
        }
    }
}
