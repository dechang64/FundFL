use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::info;

use crate::fund_db::{FundDb, Fund, RiskMetrics};
use crate::vector_db::VectorDb;
use crate::audit::AuditChain;

// 引入 proto 生成的类型
pub mod proto {
    tonic::include_proto!("fundfl");
}

use proto::{
    fund_service_server::{FundService, FundServiceServer},
    vector_service_server::{VectorService, VectorServiceServer},
    audit_service_server::{AuditService, AuditServiceServer},
    *,
};

/// gRPC 服务，聚合所有子服务
pub struct FundFlService {
    fund_db: Arc<FundDb>,
    vector_db: Arc<std::sync::RwLock<VectorDb>>,
    audit: Arc<AuditChain>,
}

impl FundFlService {
    pub fn new(
        fund_db: Arc<FundDb>,
        vector_db: Arc<std::sync::RwLock<VectorDb>>,
        audit: Arc<AuditChain>,
    ) -> Self {
        Self { fund_db, vector_db, audit }
    }

    pub fn into_server(self) -> FundServiceServer<Self> {
        FundServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl FundService for FundFlService {
    async fn get_fund(&self, req: Request<GetFundRequest>) -> Result<Response<FundInfo>, Status> {
        let code = req.into_inner().fund_code;
        info!("gRPC: GetFund({})", code);

        let fund = self.fund_db.get_fund(&code)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found(format!("Fund {} not found", code)))?;

        Ok(Response::new(FundInfo {
            code: fund.code,
            name: fund.name,
            category: fund.category,
            category_cn: fund.category_cn,
            manager: fund.manager,
            company: fund.company,
            nav: fund.nav,
            acc_nav: fund.acc_nav,
            data_months: fund.data_months,
        }))
    }

    async fn get_risk_metrics(&self, req: Request<GetRiskRequest>) -> Result<Response<RiskMetricsMsg>, Status> {
        let req = req.into_inner();
        info!("gRPC: GetRiskMetrics({})", req.fund_code);

        let risk = self.fund_db.get_risk_metrics(&req.fund_code)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found(format!("Risk metrics for {} not found", req.fund_code)))?;

        Ok(Response::new(RiskMetricsMsg {
            fund_code: risk.fund_code,
            ann_return: risk.ann_return,
            ann_vol: risk.ann_vol,
            sharpe: risk.sharpe,
            sortino: risk.sortino,
            max_drawdown: risk.max_drawdown,
            beta: risk.beta,
            alpha: risk.alpha,
            treynor: risk.treynor,
            info_ratio: risk.info_ratio,
            calmar: risk.calmar,
            var_95: risk.var_95,
            cvar_95: risk.cvar_95,
            m2: risk.m2,
            skewness: risk.skewness,
            kurtosis: risk.kurtosis,
            win_rate: risk.win_rate,
        }))
    }

    async fn search_similar(&self, req: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        let req = req.into_inner();
        info!("gRPC: SearchSimilar({}, k={})", req.fund_code, req.k);

        // 获取目标基金的风险指标，构造查询向量
        let risk = self.fund_db.get_risk_metrics(&req.fund_code)
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found(format!("Fund {} has no risk metrics", req.fund_code)))?;

        let query = risk_to_vector(&risk);
        let k = if req.k > 0 { req.k as usize } else { 5 };

        let vdb = self.vector_db.read().unwrap();
        let results = vdb.search(&query, k + 1) // +1 因为包含自身
            .map_err(|e| Status::internal(e.to_string()))?;

        // 过滤掉自身
        let similar: Vec<_> = results
            .into_iter()
            .filter(|r| r.id != req.fund_code)
            .take(k)
            .map(|r| SimilarFund {
                fund_code: r.id,
                distance: r.distance,
                fund_name: r.meta.get("name").cloned().unwrap_or_default(),
                category: r.meta.get("category").cloned().unwrap_or_default(),
                sharpe: r.meta.get("sharpe").and_then(|s| s.parse().ok()).unwrap_or(0.0),
                alpha: r.meta.get("alpha").and_then(|s| s.parse().ok()).unwrap_or(0.0),
            })
            .collect();

        // 记录审计
        let _ = self.audit.append(
            "search_similar",
            &format!("fund={}, k={}, results={}", req.fund_code, k, similar.len()),
        );

        Ok(Response::new(SearchResponse { similar }))
    }

    async fn list_funds(&self, req: Request<ListFundsRequest>) -> Result<Response<ListFundsResponse>, Status> {
        let req = req.into_inner();
        info!("gRPC: ListFunds(page={}, size={})", req.page, req.page_size);

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 20 };

        let (funds, total) = self.fund_db.list_funds(
            if req.category.is_empty() { None } else { Some(&req.category) },
            &req.sort_by,
            &req.sort_order,
            page,
            page_size,
        ).map_err(|e| Status::internal(e.to_string()))?;

        let fund_infos: Vec<FundInfo> = funds.into_iter().map(|f| FundInfo {
            code: f.code, name: f.name, category: f.category,
            category_cn: f.category_cn, manager: f.manager, company: f.company,
            nav: f.nav, acc_nav: f.acc_nav, data_months: f.data_months,
        }).collect();

        Ok(Response::new(ListFundsResponse {
            funds: fund_infos,
            total: total as i32,
            page,
            page_size,
        }))
    }

    async fn get_stats(&self, _req: Request<StatsRequest>) -> Result<Response<StatsResponse>, Status> {
        info!("gRPC: GetStats");

        let (total_funds, categories) = self.fund_db.get_stats()
            .map_err(|e| Status::internal(e.to_string()))?;

        let vdb = self.vector_db.read().unwrap();
        let (valid, chain_len, latest_hash) = self.audit.verify_chain()
            .unwrap_or((false, 0, String::new()));

        Ok(Response::new(StatsResponse {
            total_funds: total_funds as i64,
            total_vectors: vdb.len() as i64,
            vector_dimension: vdb.len() as i32, // TODO: expose actual dimension
            categories: categories.into_iter().map(|c| CategoryStatsMsg {
                category: c.category,
                category_cn: c.category_cn,
                count: c.count,
                avg_sharpe: c.avg_sharpe,
                avg_alpha: c.avg_alpha,
            }).collect(),
            audit_chain_length: chain_len,
            audit_chain_valid: valid,
        }))
    }
}

/// 将风险指标转换为20维向量
fn risk_to_vector(risk: &RiskMetrics) -> Vec<f32> {
    vec![
        risk.ann_return as f32,
        risk.ann_vol as f32,
        risk.sharpe as f32,
        risk.sortino as f32,
        risk.max_drawdown as f32,
        risk.beta as f32,
        risk.alpha as f32,
        risk.treynor as f32,
        risk.info_ratio as f32,
        risk.calmar as f32,
        risk.var_95 as f32,
        risk.cvar_95 as f32,
        risk.m2 as f32,
        risk.skewness as f32,
        risk.kurtosis as f32,
        risk.win_rate as f32,
        // 预留4维给因子暴露（MVP暂用0填充）
        0.0, 0.0, 0.0, 0.0,
    ]
}
