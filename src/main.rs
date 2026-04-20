use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

mod fund_db;
mod vector_db;
mod hnsw_index;
mod audit;
mod grpc_service;
mod rest_api;
mod web_dashboard;

use fund_db::FundDb;
use vector_db::VectorDb;
use audit::AuditChain;
use grpc_service::FundFlService;
use rest_api::ApiState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fundfl=info,tower_http=debug".parse().unwrap())
        )
        .init();

    info!("FundFL v0.1.0 — 开源私募基金分析平台");
    info!("Initializing components...");

    // 初始化数据库
    let fund_db = Arc::new(FundDb::new(std::path::Path::new("data/fundfl.db"))?);
    info!("FundDB initialized");

    // 初始化向量数据库（20维）
    let vector_db = Arc::new(std::sync::RwLock::new(VectorDb::new(20)));
    info!("VectorDB initialized (dimension=20)");

    // 初始化审计链
    let audit = Arc::new(AuditChain::new(std::path::Path::new("data/audit.db"))?);
    info!("AuditChain initialized");

    // 记录启动审计
    audit.append("system_start", "FundFL v0.1.0 started")?;

    // gRPC 服务
    let grpc_service = FundFlService::new(
        Arc::clone(&fund_db),
        Arc::clone(&vector_db),
        Arc::clone(&audit),
    );

    // REST API
    let api_state = ApiState {
        fund_db: Arc::clone(&fund_db),
        vector_db: Arc::clone(&vector_db),
        audit: Arc::clone(&audit),
    };

    // 启动 gRPC 服务（端口 50051）
    let grpc_addr = "[::]:50051".parse()?;
    let grpc_server = tonic::transport::Server::builder()
        .add_service(grpc_service.into_server())
        .serve(grpc_addr);

    // 启动 REST + Web Dashboard（端口 8080）
    let rest_app = rest_api::create_router(api_state)
        .merge(web_dashboard::create_dashboard());
    let rest_listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    let rest_server = axum::serve(rest_listener, rest_app);

    info!("gRPC server ready on 0.0.0.0:50051");
    info!("REST server ready on 0.0.0.0:8080");
    info!("Web dashboard: http://0.0.0.0:8080");
    info!("Run `python python/load_data.py` to load fund data");

    // 并行运行 gRPC 和 REST
    tokio::select! {
        r = grpc_server => { r?; }
        r = rest_server => { r?; }
    }

    Ok(())
}
