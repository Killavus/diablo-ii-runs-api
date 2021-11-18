use std::{convert::Infallible, str::FromStr};
use thiserror::Error;
use warp::{body::BodyDeserializeError, hyper::StatusCode, reject::PayloadTooLarge, Rejection};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString};
use warp::Filter;

#[derive(sqlx::Type, EnumString, AsRefStr, Serialize)]
#[sqlx(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case", tag = "type")]
enum RunTarget {
    Pit,
    Andariel,
    Cows,
    Countess,
    ArcaneSanctuary,
    AncientTunnels,
    Travincal,
    Mephisto,
    Chaos,
    Pindle,
    Shenk,
    Eldritch,
    Worldstone,
    Baal,
}

#[derive(Error, Debug)]
enum ApiError {
    #[error("Database connection error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Data format error")]
    DataFormatError(#[from] strum::ParseError),
}

impl warp::reject::Reject for ApiError {}

#[derive(sqlx::FromRow, Serialize)]
struct Run {
    id: i32,
    #[serde(flatten)]
    target: RunTarget,
    ran_at: DateTime<chrono::Utc>,
}

use sqlx::Postgres;

#[derive(Deserialize)]
struct CreatePayload {
    target: String,
}

pub fn root(
    db_pool: sqlx::Pool<Postgres>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone {
    warp::path("api")
        .and(list(db_pool.clone()).or(create(db_pool.clone())))
        .recover(api_error_handler)
}

fn create(
    db_pool: sqlx::Pool<Postgres>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path!("runs" / String))
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .and_then(move |scope, payload| handle_create(db_pool.clone(), scope, payload))
}

async fn handle_create(
    db_pool: sqlx::Pool<Postgres>,
    scope: String,
    payload: CreatePayload,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut conn = db_pool.acquire().await.map_err(|err| ApiError::from(err))?;
    let target = RunTarget::from_str(&payload.target).map_err(|err| ApiError::from(err))?;

    let record: Run = sqlx::query_as!(
        Run,
        r#"INSERT INTO runs (target, scope) VALUES ($1, $2) RETURNING id, target as "target: RunTarget", ran_at"#,
        target.as_ref(),
        scope,
    )
    .fetch_one(&mut conn)
    .await.map_err(|err| ApiError::from(err))?;

    Ok(warp::reply::json(&record))
}

fn list(
    db_pool: sqlx::Pool<Postgres>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("runs" / String))
        .and_then(move |scope| handle_list(db_pool.clone(), scope))
}

async fn handle_list(
    db_pool: sqlx::Pool<Postgres>,
    scope: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut conn = db_pool.acquire().await.map_err(|err| ApiError::from(err))?;

    let record: Vec<Run> = sqlx::query_as!(
        Run,
        r#"SELECT id, target as "target: RunTarget", ran_at FROM runs WHERE scope = $1"#,
        scope
    )
    .fetch_all(&mut conn)
    .await
    .map_err(|err| ApiError::from(err))?;

    Ok(warp::reply::json(&record))
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u32,
    message: String,
}

async fn api_error_handler(err: Rejection) -> Result<impl warp::Reply, Infallible> {
    let mut http_code: Option<StatusCode> = None;
    let code;
    let message;

    tracing::error!("error while processing request: {:?}", err);

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND.as_u16() as u32;
        message = "not found".into();
    } else if let Some(_) = err.find::<BodyDeserializeError>() {
        code = StatusCode::UNPROCESSABLE_ENTITY.as_u16() as u32;
        message = "failed to process request body".into();
    } else if let Some(_) = err.find::<PayloadTooLarge>() {
        code = StatusCode::BAD_REQUEST.as_u16() as u32;
        message = "request body is too large".into();
    } else if let Some(err) = err.find::<ApiError>() {
        use ApiError::*;

        match err {
            DatabaseError(error) => {
                let parsed = database_error_handler(error);
                code = parsed.0;
                message = parsed.1;
                http_code = api_code_to_http(code);
            }
            DataFormatError(error) => {
                let parsed = data_format_error_handler(error);
                code = parsed.0;
                message = parsed.1;
                http_code = api_code_to_http(code);
            }
        };
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR.as_u16() as u32;
        message = "internal server error".into();
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorMessage {
            code: code as u32,
            message,
        }),
        http_code
            .ok_or(())
            .or_else(|_| StatusCode::try_from(code.min(u16::MAX as u32) as u16))
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
    ))
}

fn api_code_to_http(code: u32) -> Option<StatusCode> {
    let prefix = code / 1000;
    match prefix {
        20..=29 => Some(StatusCode::UNPROCESSABLE_ENTITY),
        30..=39 => Some(StatusCode::UNAUTHORIZED),
        40..=49 => Some(StatusCode::FORBIDDEN),
        _ => Some(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn database_error_handler(err: &sqlx::Error) -> (u32, String) {
    use sqlx::Error;

    match err {
        err @ Error::Decode(_) | err @ Error::ColumnDecode { .. } => (20001, format!("{:?}", err)),
        _ => (80000, "internal server error".into()),
    }
}

fn data_format_error_handler(err: &strum::ParseError) -> (u32, String) {
    use strum::ParseError as Error;

    match err {
        Error::VariantNotFound => (20001, format!("{:?}", err)),
    }
}
