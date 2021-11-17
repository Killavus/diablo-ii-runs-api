use std::{convert::Infallible, sync::Arc};

use nanoid::nanoid;
use warp::{http::HeaderValue, hyper::header::HeaderName, Filter};

fn request_id() -> String {
    nanoid!(16, &nanoid::alphabet::SAFE, nanoid::rngs::non_secure)
}
pub struct HandlerCtx {
    request_id: String,
}

impl HandlerCtx {
    fn init() -> Self {
        Self {
            request_id: request_id(),
        }
    }

    pub fn request_id_wrapper<T>(
        f: impl warp::Filter<Extract = (T,), Error = Infallible> + Clone + Send + Sync + 'static,
    ) -> impl warp::Filter<Extract = (warp::reply::WithHeader<T>,), Error = Infallible>
           + Clone
           + Send
           + Sync
           + 'static
    where
        T: warp::Reply,
    {
        warp::any()
            .map(|| {
                let ctx = Arc::new(HandlerCtx::init());
                tracing::Span::current().record("request_id", &ctx.clone().request_id.as_str());
                ctx
            })
            .and(f)
            .map(|ctx: Arc<HandlerCtx>, r: T| {
                warp::reply::with_header(
                    r,
                    HeaderName::from_static("x-request-id"),
                    HeaderValue::from_str(&ctx.clone().request_id).unwrap(),
                )
            })
    }
}
