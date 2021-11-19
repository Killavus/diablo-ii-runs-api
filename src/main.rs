use dotenv::dotenv;
use std::net::IpAddr;
use warp::{
    hyper::{header, Method},
    wrap_fn, Filter,
};

mod api;
mod context;
mod database;
mod trace;

pub type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn app_trace(info: warp::trace::Info) -> tracing::Span {
    tracing::info_span!(
        "request",
        method = %info.method(),
        path = %info.path(),
        request_id = tracing::field::Empty
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    trace::setup()?;
    dotenv().ok();
    let handler_ctx = wrap_fn(context::HandlerCtx::request_id_wrapper);
    let db_pool = database::pool().await?;
    let origin_list =
        std::env::var("RUNS_API_CORS_ORIGINS").unwrap_or("http://localhost:3000".into());

    let mut cors = warp::cors()
        .allow_headers(vec![header::CONTENT_TYPE])
        .allow_methods(vec![Method::POST, Method::GET]);

    for origin in origin_list.split(",") {
        let origin = origin.trim();
        cors = cors.allow_origin(origin);
    }

    let routes = api::root(db_pool)
        .with(handler_ctx)
        .with(warp::trace::trace(app_trace))
        .with(cors);

    let listen_addr: IpAddr = std::env::var("RUST_API_LISTEN_ADDR")
        .unwrap_or("127.0.0.1".into())
        .parse()?;

    let listen_port = std::env::var("RUST_API_LISTEN_PORT")
        .unwrap_or("8888".into())
        .parse()?;

    warp::serve(routes).run((listen_addr, listen_port)).await;
    Ok(())
}
