//! 联邦基金分析 Agent
//!
//! 基于 ReAct 模式，专为基金分析场景定制：
//! - 任务规划器：将分析任务分解为工具调用链
//! - 工具调用器：调度向量搜索 / FL查询 / 研报复现
//! - 反思器：检测执行偏差，自我修正

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────
// 工具定义
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ToolName {
    VectorSearch,      // 基金相似性搜索
    RiskAnalysis,     // 风险指标分析
    FedLearningQuery, // 联邦学习查询
    HallucinationCheck, // 幻觉检测
    ReportGenerate,   // 分析报告生成
    ChartGenerate,   // 图表生成
    Reflect,          // 反思
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolName::VectorSearch => write!(f, "VectorSearch"),
            ToolName::RiskAnalysis => write!(f, "RiskAnalysis"),
            ToolName::FedLearningQuery => write!(f, "FedLearningQuery"),
            ToolName::HallucinationCheck => write!(f, "HallucinationCheck"),
            ToolName::ReportGenerate => write!(f, "ReportGenerate"),
            ToolName::ChartGenerate => write!(f, "ChartGenerate"),
            ToolName::Reflect => write!(f, "Reflect"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: ToolName,
    pub params: HashMap<String, String>,
    pub output: Option<String>,
    pub success: bool,
    pub duration_ms: u64,
}

// ──────────────────────────────────────────────────────────────
// 规划步骤
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: u32,
    pub tool: ToolName,
    pub reasoning: String,
    pub params: HashMap<String, String>,
    pub expected_output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub task_id: String,
    pub steps: Vec<PlanStep>,
    pub confidence: f32,
}

// ──────────────────────────────────────────────────────────────
// 执行结果
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub final_answer: String,
    pub steps_taken: Vec<ToolCall>,
    pub reflections: Vec<Reflection>,
    pub overall_confidence: f32,
    pub total_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reflection {
    pub step: u32,
    pub question: String,
    pub improvement: Option<String>,
}

// ──────────────────────────────────────────────────────────────
// 基金分析 Agent
// ──────────────────────────────────────────────────────────────

pub struct FundAnalysisAgent {
    task_counter: u64,
    max_steps: u32,
    /// 模拟基金数据库（生产环境接真实数据）
    fund_db: HashMap<String, FundRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundRecord {
    pub code: String,
    pub name: String,
    pub sharpe: f32,
    pub ann_return: f32,
    pub max_drawdown: f32,
    pub category: String,
}

impl Default for FundAnalysisAgent {
    fn default() -> Self {
        let mut agent = Self::new();
        // 预置演示数据
        agent.add_demo_funds();
        agent
    }
}

impl FundAnalysisAgent {
    pub fn new() -> Self {
        Self {
            task_counter: 0,
            max_steps: 10,
            fund_db: HashMap::new(),
        }
    }

    pub fn set_max_steps(&mut self, max: u32) {
        self.max_steps = max;
    }

    fn add_demo_funds(&mut self) {
        let funds = vec![
            ("HF_ALPHA_001", "Alpha绝对收益", 1.42, 18.3, -8.5, "对冲"),
            ("HF_BETA_002",  "Beta灵活配置", 0.87, 12.1, -15.2, "对冲"),
            ("HF_MACRO_003", "宏观策略", 1.15, 22.7, -12.3, "宏观"),
            ("HF_CTA_004",   "CTA趋势", 0.93, 14.8, -22.1, "CTA"),
            ("HF arb_005",   "统计套利", 1.68, 9.2, -4.1, "套利"),
        ];
        for (code, name, sharpe, ret, dd, cat) in funds {
            self.fund_db.insert(code.to_string(), FundRecord {
                code: code.to_string(),
                name: name.to_string(),
                sharpe,
                ann_return: ret,
                max_drawdown: dd,
                category: cat.to_string(),
            });
        }
    }

    fn next_task_id(&mut self) -> String {
        self.task_counter += 1;
        format!("fund_task_{}", self.task_counter)
    }

    /// 分析任务，返回执行计划
    pub fn plan(&mut self, task: &str) -> Plan {
        let task_id = self.next_task_id();
        let t = task.to_lowercase();
        let mut steps = Vec::new();
        let mut step_id = 1u32;

        // 规则型规划
        if t.contains("相似") || t.contains("找") || t.contains("推荐") {
            steps.push(PlanStep {
                step_id,
                tool: ToolName::VectorSearch,
                reasoning: "任务涉及基金相似性检索，使用HNSW向量搜索".to_string(),
                params: [("task".to_string(), task.to_string())].into(),
                expected_output: "Top-K 相似基金列表".to_string(),
            });
            step_id += 1;
        }

        if t.contains("风险") || t.contains("收益") || t.contains("分析")
            || t.contains("比较") || t.contains("评估")
        {
            steps.push(PlanStep {
                step_id,
                tool: ToolName::RiskAnalysis,
                reasoning: "任务涉及风险/收益分析，使用多因子风险模型".to_string(),
                params: [("task".to_string(), task.to_string())].into(),
                expected_output: "风险指标报告".to_string(),
            });
            step_id += 1;
        }

        if t.contains("联邦") || t.contains("多家") || t.contains("协作") {
            steps.push(PlanStep {
                step_id,
                tool: ToolName::FedLearningQuery,
                reasoning: "任务涉及跨机构分析，使用联邦学习查询".to_string(),
                params: [("task".to_string(), task.to_string())].into(),
                expected_output: "联邦聚合结果".to_string(),
            });
            step_id += 1;
        }

        // 任何分析任务最后都做幻觉检测
        if !steps.is_empty() {
            steps.push(PlanStep {
                step_id,
                tool: ToolName::HallucinationCheck,
                reasoning: "对分析结论进行五层幻觉防御检测".to_string(),
                params: [("context".to_string(), "[前一步结果]".to_string())].into(),
                expected_output: "幻觉风险评估".to_string(),
            });
            step_id += 1;

            steps.push(PlanStep {
                step_id,
                tool: ToolName::ReportGenerate,
                reasoning: "生成结构化分析报告".to_string(),
                params: [("format".to_string(), "structured".to_string())].into(),
                expected_output: "完整分析报告".to_string(),
            });
            step_id += 1;
        }

        if steps.is_empty() {
            steps.push(PlanStep {
                step_id: 1,
                tool: ToolName::VectorSearch,
                reasoning: "通用基金查询".to_string(),
                params: [("query".to_string(), task.to_string())].into(),
                expected_output: "查询结果".to_string(),
            });
        }

        steps.push(PlanStep {
            step_id: steps.len() as u32 + 1,
            tool: ToolName::Reflect,
            reasoning: "执行完毕，进行反思和改进".to_string(),
            params: [("task".to_string(), task.to_string())].into(),
            expected_output: "反思记录".to_string(),
        });

        Plan {
            task_id,
            steps,
            confidence: 0.8,
        }
    }

    /// 模拟执行单个工具
    fn execute_tool(&self, tool: &ToolName, params: &HashMap<String, String>) -> (String, bool) {
        match tool {
            ToolName::VectorSearch => {
                let task = params.get("task").cloned().unwrap_or_default();
                let results: Vec<String> = self.fund_db.values()
                    .filter(|f| {
                        f.name.contains(&task) || f.code.contains(&task)
                            || f.category.contains(&task)
                    })
                    .take(3)
                    .map(|f| format!(
                        "{} [{}]: Sharpe={:.2}, 收益={:.1}%, 最大回撤={:.1}%",
                        f.code, f.name, f.sharpe, f.ann_return, f.max_drawdown
                    ))
                    .collect();
                if results.is_empty() {
                    (format!("未找到匹配「{}」的基金，返回全部演示数据:\n{}\n[3只相似基金]",
                        task,
                        self.fund_db.values().take(3)
                            .map(|f| format!("{}: Sharpe={:.2}", f.code, f.sharpe))
                            .collect::<Vec<_>>().join("\n")
                    ), true)
                } else {
                    (format!("找到 {} 只相似基金:\n{}", results.len(), results.join("\n")), true)
                }
            }
            ToolName::RiskAnalysis => {
                let task = params.get("task").cloned().unwrap_or_default();
                let analysis = format!(
                    "【风险分析】\n\
                    基于任务「{}」的分析：\n\
                    • Sharpe比率: 范围 0.87-1.68，中位数 1.15\n\
                    • 年化收益: 范围 9.2%-22.7%，均值为 15.4%\n\
                    • 最大回撤: 范围 -4.1%~-22.1%，CTA策略回撤最大\n\
                    • 风险调整收益最优: HF_ALPHA_001 (Sharpe=1.42)\n\
                    • 低回撤首选: HF arb_005 (最大回撤仅-4.1%)",
                    task
                );
                (analysis, true)
            }
            ToolName::FedLearningQuery => {
                (format!(
                    "【联邦学习查询】\n\
                    参与机构: 3家对冲基金\n\
                    本轮聚合: FedAvg已完成 (Round 5)\n\
                    全局Sharpe预测: 1.23 (↑ 8.2% vs 本地模型)\n\
                    共识风险因子: momentum_30d拥挤度中等\n\
                    隐私保护: 本地差分隐私 (ε=2.0)"
                ), true)
            }
            ToolName::HallucinationCheck => {
                let ctx = params.get("context").cloned().unwrap_or_default();
                let risk = if ctx.contains("最优") || ctx.contains("最高") { 0.23 } else { 0.31 };
                (format!(
                    "【幻觉防御检测】\n\
                    风险评分: {:.0%}\n\
                    判定: {}\n\
                    防御动作: {}\n\
                    触发层: 无（结论可信）",
                    risk,
                    if risk < 0.5 { "LikelyTrue" } else { "Uncertain" },
                    if risk < 0.5 { "Accept" } else { "Flag" }
                ), true)
            }
            ToolName::ReportGenerate => {
                (format!(
                    "【基金分析报告】\n\
                    生成时间: {}\n\
                    分析维度: 风险收益 · 因子暴露 · 联邦对比\n\
                    置信度: 81%\n\
                    建议: 详见上方分析结果",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M")
                ), true)
            }
            ToolName::Reflect => {
                ("【反思】\n执行完成，未检测到明显偏差。置信度 81%，建议关注多节点一致性验证。".to_string(), true)
            }
            ToolName::ChartGenerate => {
                ("【图表】已生成：收益曲线、风险散点图、有效前沿".to_string(), true)
            }
        }
    }

    /// 执行任务
    pub fn run(&mut self, task: &str) -> ExecutionResult {
        let start = std::time::Instant::now();
        let plan = self.plan(task);

        let mut steps_taken = Vec::new();
        let mut reflections = Vec::new();
        let mut confidence = 1.0_f32;

        for step in &plan.steps {
            if steps_taken.len() >= self.max_steps as usize {
                reflections.push(Reflection {
                    step: steps_taken.len() as u32,
                    question: "超出最大步数限制？".to_string(),
                    improvement: Some("考虑将任务分解为更小的子任务".to_string()),
                });
                break;
            }

            let step_start = std::time::Instant::now();
            let (output, success) = self.execute_tool(&step.tool, &step.params);
            confidence *= if success { 0.9 } else { 0.5 };

            steps_taken.push(ToolCall {
                tool: step.tool.clone(),
                params: step.params.clone(),
                output: Some(output.clone()),
                success,
                duration_ms: step_start.elapsed().as_millis() as u64,
            });

            if !success {
                reflections.push(Reflection {
                    step: step.step_id,
                    question: format!("步骤 {:?} 失败", step.tool),
                    improvement: Some("检查参数或使用替代工具".to_string()),
                });
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let final_answer = steps_taken.last()
            .and_then(|s| s.output.clone())
            .unwrap_or_else(|| "任务执行完成".to_string());

        ExecutionResult {
            task_id: plan.task_id,
            success: steps_taken.iter().all(|s| s.success),
            final_answer,
            steps_taken,
            reflections,
            overall_confidence: confidence.max(0.0).min(1.0),
            total_duration_ms: duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_similarity_task() {
        let mut agent = FundAnalysisAgent::new();
        let plan = agent.plan("找出与HF_ALPHA_001最相似的3只基金");
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.steps[0].tool, ToolName::VectorSearch);
    }

    #[test]
    fn test_plan_risk_task() {
        let mut agent = FundAnalysisAgent::new();
        let plan = agent.plan("分析这些基金的风险调整后收益");
        let tools: Vec<_> = plan.steps.iter().map(|s| &s.tool).collect();
        assert!(tools.contains(&&ToolName::RiskAnalysis));
    }

    #[test]
    fn test_run_task() {
        let mut agent = FundAnalysisAgent::new();
        let result = agent.run("HF_ALPHA_001的风险收益分析");
        assert!(!result.steps_taken.is_empty());
        assert!(result.total_duration_ms > 0);
        assert!(result.overall_confidence > 0.0);
    }

    #[test]
    fn test_demo_funds_loaded() {
        let agent = FundAnalysisAgent::new();
        assert_eq!(agent.fund_db.len(), 5);
        assert!(agent.fund_db.contains_key("HF_ALPHA_001"));
    }
}
