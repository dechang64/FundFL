use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

mod fund_db;
mod vector_db;
mod hnsw_index;
mod audit;
mod hallucination;
mod fed_learn;
mod agent;
mod grpc_service;
mod rest_api;
mod web_dashboard;

use fund_db::FundDb;
use vector_db::VectorDb;
use audit::AuditChain;
use hallucination::DefenseConfig;
use fed_learn::{FedAvgAggregator, FLConfig, FLNode, ClientUpdate};
use agent::FundAnalysisAgent;
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

    info!("FundFL v0.2.0 — 联邦智能基金分析平台");
    info!("Initializing components...");

    // ── 核心组件 ──
    let fund_db = Arc::new(FundDb::new(std::path::Path::new("data/fundfl.db"))?);
    info!("✅ FundDB initialized");

    let vector_db = Arc::new(std::sync::RwLock::new(VectorDb::new(20)));
    info!("✅ VectorDB initialized (dimension=20)");

    let audit = Arc::new(AuditChain::new(std::path::Path::new("data/audit.db"))?);
    info!("✅ AuditChain initialized");

    audit.append("system_start", "FundFL v0.2.0 started — 联邦智能基金分析平台")?;

    // ── 联邦学习组件 ──
    let fl_config = FLConfig {
        num_rounds: 10,
        min_clients: 2,
        client_fraction: 1.0,
        local_epochs: 5,
        learning_rate: 0.01,
        enable_dp: true,
        dp_epsilon: 2.0,
        enable_task_aware: true,
    };
    let mut fl_aggregator = FedAvgAggregator::new(fl_config).with_audit(Arc::clone(&audit));

    // 注册联邦节点（演示）
    fl_aggregator.register_node(FLNode::new("hf_alpha", "Alpha对冲基金", 5000));
    fl_aggregator.register_node(FLNode::new("hf_beta", "Beta资产管理", 8000));
    fl_aggregator.register_node(FLNode::new("hf_gamma", "Gamma私募", 3200));

    // 初始化全局模型（20维 = 向量库维度）
    fl_aggregator.initialize_model(20, 1);
    info!("✅ Federated Learning initialized (3 nodes, DP enabled ε=2.0)");

    // ── 幻觉防御配置 ──
    let defense_config = DefenseConfig::default();
    info!("✅ Hallucination Defense initialized (5-layer)");

    // ── Agent ──
    let mut fund_agent = FundAnalysisAgent::new();
    fund_agent.set_max_steps(10);
    info!("✅ Fund Analysis Agent initialized");

    // ── 运行演示场景 ──
    run_demo(&fl_aggregator, &mut fund_agent)?;

    // ── gRPC 服务 ──
    let grpc_service = FundFlService::new(
        Arc::clone(&fund_db),
        Arc::clone(&vector_db),
        Arc::clone(&audit),
    );

    // ── REST API ──
    let api_state = ApiState {
        fund_db: Arc::clone(&fund_db),
        vector_db: Arc::clone(&vector_db),
        audit: Arc::clone(&audit),
    };

    // 启动 gRPC（端口 50051）
    let grpc_addr = "[::]:50051".parse()?;
    let grpc_server = tonic::transport::Server::builder()
        .add_service(grpc_service.into_server())
        .serve(grpc_addr);

    // 启动 REST + Dashboard（端口 8080）
    let rest_app = rest_api::create_router(api_state)
        .merge(web_dashboard::create_dashboard());
    let rest_listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    let rest_server = axum::serve(rest_listener, rest_app);

    info!("gRPC server ready on 0.0.0.0:50051");
    info!("REST server ready on 0.0.0.0:8080");
    info!("Run `python python/load_data.py` to load fund data");

    tokio::select! {
        r = grpc_server => { r?; }
        r = rest_server => { r?; }
    }

    Ok(())
}

fn run_demo(
    fl: &FedAvgAggregator,
    agent: &mut FundAnalysisAgent,
) -> anyhow::Result<()> {
    use std::collections::HashMap;

    println!("\n{}", "═".repeat(70));
    println!("  FundFL v0.2.0 — 联邦智能基金分析平台");
    println!("{}", "═".repeat(70));

    // ── 联邦学习演示 ──
    println!("\n🤝 联邦学习");
    println!("{}", "-".repeat(50));
    println!("  参与节点: {} 家", fl.online_nodes());
    for node in fl.get_nodes() {
        println!("    • {} ({}): {} 样本", node.node_id, node.institution, node.data_size);
    }

    let history = fl.history();
    if history.is_empty() {
        println!("  状态: 等待训练数据输入");
    } else {
        for record in history {
            println!("  Round {}: loss={:.4}, 参与={}, 耗时={}ms, 隐私={}",
                record.round, record.global_loss, record.participants.len(),
                record.aggregation_time_ms, record.privacy_mechanism);
        }
    }

    // ── Agent 分析演示 ──
    println!("\n🤖 Agent 分析");
    println!("{}", "-".repeat(50));

    let tasks = vec![
        "分析 HF_ALPHA_001 的风险调整后收益",
        "找出与 Alpha 策略最相似的 3 只基金",
        "比较宏观策略和 CTA 策略的风险收益特征",
    ];

    for task in &tasks {
        println!("\n  任务: {}", task);
        let result = agent.run(task);
        println!("  执行: {} 步, 置信度: {:.0%}, 耗时: {}ms",
            result.steps_taken.len(), result.overall_confidence, result.total_duration_ms);
        for step in &result.steps_taken {
            let icon = if step.success { "✅" } else { "❌" };
            println!("    {} {:?} [{}ms]", icon, step.tool, step.duration_ms);
        }
        if !result.reflections.is_empty() {
            for r in &result.reflections {
                if let Some(imp) = &r.improvement {
                    println!("    💭 Step {}: {}", r.step, imp);
                }
            }
        }
    }

    println!("\n{}", "═".repeat(70));
    println!("  联邦平台演示完成");
    println!("{}", "═".repeat(70));

    Ok(())
}
