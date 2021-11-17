use dotenv::dotenv;
use warp::{wrap_fn, Filter};

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

    let routes = api::root(db_pool)
        .with(handler_ctx)
        .with(warp::trace::trace(app_trace));

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await;
    Ok(())
}
