use actix_web::{HttpRequest, HttpResponse, Responder};

// NOTE: All arguments of a request handler must implement the
// `FromRequest` trait. Its method `from_request` will be invoked
// for each argument. Should all extractions succeed, then the body
// of the handler will be executed.
pub async fn health_check(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
}
