use dotenv::dotenv;
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

    let cors = warp::cors()
        .allow_headers(vec![header::CONTENT_TYPE])
        .allow_methods(vec![Method::POST, Method::GET])
        .allow_origin("http://localhost:3000")
        .build();

    let routes = api::root(db_pool)
        .with(handler_ctx)
        .with(warp::trace::trace(app_trace))
        .with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await;
    Ok(())
}
