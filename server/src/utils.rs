use actix_cors::Cors;
use actix_web::http::header;

pub fn create_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|_origin, _req_head| {
            // origin.as_bytes().ends_with(b"ore.supply") || // Production origin
            // origin == "http://localhost:8080" // Local development origin
            true
        })
        .allowed_methods(vec!["GET", "POST"]) // Methods you want to allow
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
}
