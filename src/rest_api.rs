use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::fund_db::FundDb;
use crate::vector_db::VectorDb;
use crate::audit::AuditChain;

/// REST API 状态
#[derive(Clone)]
pub struct ApiState {
    pub fund_db: Arc<FundDb>,
    pub vector_db: Arc<std::sync::RwLock<VectorDb>>,
    pub audit: Arc<AuditChain>,
}

/// REST API 错误响应
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// 基金详情响应
#[derive(Serialize)]
pub struct FundDetailResponse {
    pub fund: serde_json::Value,
    pub risk: serde_json::Value,
}

/// 相似基金响应
#[derive(Serialize)]
pub struct SimilarResponse {
    pub query: String,
    pub similar: Vec<serde_json::Value>,
}

/// 统计响应
#[derive(Serialize)]
pub struct StatsResponse {
    pub total_funds: i64,
    pub total_vectors: usize,
    pub categories: Vec<serde_json::Value>,
    pub audit_chain_length: i64,
    pub audit_chain_valid: bool,
}

/// 分页查询参数
#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

/// 创建 REST API 路由
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/funds", get(list_funds))
        .route("/api/v1/funds/{code}", get(get_fund))
        .route("/api/v1/funds/{code}/risk", get(get_risk))
        .route("/api/v1/funds/{code}/similar", get(search_similar))
        .route("/api/v1/stats", get(get_stats))
        .route("/api/v1/audit/logs", get(get_audit_logs))
        .route("/api/v1/audit/verify", get(verify_audit))
        .with_state(state)
}

/// 健康检查
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "FundFL",
        "version": "0.1.0"
    }))
}

/// 基金列表
async fn list_funds(
    State(state): State<ApiState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    info!("REST: ListFunds({:?})", q);

    let (funds, total) = state.fund_db.list_funds(
        q.category.as_deref(),
        q.sort_by.as_deref().unwrap_or("sharpe"),
        q.sort_order.as_deref().unwrap_or("desc"),
        q.page.unwrap_or(1),
        q.page_size.unwrap_or(20),
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    Ok(Json(serde_json::json!({
        "funds": funds,
        "total": total,
        "page": q.page.unwrap_or(1),
        "page_size": q.page_size.unwrap_or(20),
    })))
}

/// 基金详情
async fn get_fund(
    State(state): State<ApiState>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    info!("REST: GetFund({})", code);

    let fund = state.fund_db.get_fund(&code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: format!("Fund {} not found", code) })))?;

    let risk = state.fund_db.get_risk_metrics(&code).unwrap_or(None);

    Ok(Json(serde_json::json!({
        "fund": fund,
        "risk": risk,
    })))
}

/// 风险指标
async fn get_risk(
    State(state): State<ApiState>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    info!("REST: GetRisk({})", code);

    let risk = state.fund_db.get_risk_metrics(&code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: format!("Risk metrics for {} not found", code) })))?;

    Ok(Json(serde_json::json!(risk)))
}

/// 搜索相似基金
async fn search_similar(
    State(state): State<ApiState>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    info!("REST: SearchSimilar({})", code);

    let risk = state.fund_db.get_risk_metrics(&code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: format!("Fund {} has no risk metrics", code) })))?;

    let query = vec![
        risk.ann_return as f32, risk.ann_vol as f32, risk.sharpe as f32,
        risk.sortino as f32, risk.max_drawdown as f32, risk.beta as f32,
        risk.alpha as f32, risk.treynor as f32, risk.info_ratio as f32,
        risk.calmar as f32, risk.var_95 as f32, risk.cvar_95 as f32,
        risk.m2 as f32, risk.skewness as f32, risk.kurtosis as f32,
        risk.win_rate as f32, 0.0, 0.0, 0.0, 0.0,
    ];

    let vdb = state.vector_db.read().unwrap();
    let results = vdb.search(&query, 6) // +1 for self
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let similar: Vec<_> = results
        .into_iter()
        .filter(|r| r.id != code)
        .take(5)
        .map(|r| serde_json::json!({
            "fund_code": r.id,
            "distance": r.distance,
            "name": r.meta.get("name").cloned().unwrap_or_default(),
            "category": r.meta.get("category").cloned().unwrap_or_default(),
            "sharpe": r.meta.get("sharpe").and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
        }))
        .collect();

    // 审计
    let _ = state.audit.append("search_similar", &format!("fund={}, results={}", code, similar.len()));

    Ok(Json(serde_json::json!({
        "query": code,
        "similar": similar,
    })))
}

/// 统计信息
async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    info!("REST: GetStats");

    let (total_funds, categories) = state.fund_db.get_stats()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let vdb = state.vector_db.read().unwrap();
    let (valid, chain_len, _) = state.audit.verify_chain().unwrap_or((false, 0, String::new()));

    Ok(Json(serde_json::json!({
        "total_funds": total_funds,
        "total_vectors": vdb.len(),
        "categories": categories,
        "audit_chain_length": chain_len,
        "audit_chain_valid": valid,
    })))
}

/// 审计日志
async fn get_audit_logs(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let logs = state.audit.get_recent(50, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    Ok(Json(serde_json::json!({ "logs": logs })))
}

/// 验证审计链
async fn verify_audit(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let (valid, length, latest_hash) = state.audit.verify_chain()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    Ok(Json(serde_json::json!({
        "valid": valid,
        "chain_length": length,
        "latest_hash": latest_hash,
    })))
}
