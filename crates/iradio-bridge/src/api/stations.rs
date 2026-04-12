use crate::api::{ApiResult, ApiState, AppError};
use crate::radiobrowser::{Country, Tag};
use crate::state::Station;
use axum::extract::{Query, State};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub country: Option<String>,
    pub tag: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct TopQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct TagQuery {
    pub tag: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Deserialize)]
pub struct CountryQuery {
    pub country: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    40
}

/// Clone the RadioBrowserClient out of the lock so we don't hold it across await points.
async fn get_client(ctx: &ApiState) -> Result<crate::radiobrowser::RadioBrowserClient, AppError> {
    ctx.rb_client.read().await.clone().ok_or_else(|| {
        AppError(anyhow::anyhow!(
            "RadioBrowser client is initializing, retry shortly"
        ))
    })
}

pub async fn search(
    State(ctx): State<ApiState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Vec<Station>> {
    let rb = get_client(&ctx).await?;
    let results = rb
        .search(
            q.q.as_deref().unwrap_or(""),
            q.country.as_deref(),
            q.tag.as_deref(),
            q.limit,
        )
        .await?;
    Ok(axum::Json(results))
}

pub async fn top(
    State(ctx): State<ApiState>,
    Query(q): Query<TopQuery>,
) -> ApiResult<Vec<Station>> {
    let rb = get_client(&ctx).await?;
    Ok(axum::Json(rb.top(q.limit).await?))
}

pub async fn tags(State(ctx): State<ApiState>) -> ApiResult<Vec<Tag>> {
    let rb = get_client(&ctx).await?;
    Ok(axum::Json(rb.tags(200).await?))
}

pub async fn countries(State(ctx): State<ApiState>) -> ApiResult<Vec<Country>> {
    let rb = get_client(&ctx).await?;
    Ok(axum::Json(rb.countries().await?))
}

pub async fn by_tag(
    State(ctx): State<ApiState>,
    Query(q): Query<TagQuery>,
) -> ApiResult<Vec<Station>> {
    let rb = get_client(&ctx).await?;
    Ok(axum::Json(rb.stations_by_tag(&q.tag, q.limit).await?))
}

pub async fn by_country(
    State(ctx): State<ApiState>,
    Query(q): Query<CountryQuery>,
) -> ApiResult<Vec<Station>> {
    let rb = get_client(&ctx).await?;
    Ok(axum::Json(
        rb.stations_by_country(&q.country, q.limit).await?,
    ))
}
