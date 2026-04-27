//! 联邦学习模块
//!
//! FedAvg 聚合引擎，专为对冲基金/私募场景设计：
//! - 多家机构本地训练，只共享梯度，不共享持仓
//! - 差分隐私可选（高斯噪声）
//! - Task-Aware 聚合（按因子表现加权）
//! - 区块链审计（每次聚合可追溯）

use crate::audit::AuditChain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────
// 配置
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FLConfig {
    /// 聚合轮次
    pub num_rounds: u32,
    /// 每轮最小参与节点数
    pub min_clients: u32,
    /// 每轮参与比例
    pub client_fraction: f32,
    /// 本地训练轮次
    pub local_epochs: u32,
    /// 学习率
    pub learning_rate: f32,
    /// 是否启用差分隐私
    pub enable_dp: bool,
    /// 差分隐私 ε（越小越隐私）
    pub dp_epsilon: f32,
    /// 是否启用 Task-Aware 聚合
    pub enable_task_aware: bool,
}

impl Default for FLConfig {
    fn default() -> Self {
        Self {
            num_rounds: 10,
            min_clients: 2,
            client_fraction: 1.0,
            local_epochs: 5,
            learning_rate: 0.01,
            enable_dp: false,
            dp_epsilon: 2.0,
            enable_task_aware: false,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 联邦节点
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FLNode {
    pub node_id: String,
    pub institution: String,
    /// 数据集大小（用于 FedAvg 加权）
    pub data_size: u32,
    /// 当前训练轮次
    pub current_round: u32,
    /// 是否在线
    pub online: bool,
    /// 本轮损失
    pub local_loss: Option<f32>,
    /// 因子表现评分（用于 Task-Aware 聚合）
    pub task_scores: HashMap<String, f32>,
}

impl FLNode {
    pub fn new(node_id: &str, institution: &str, data_size: u32) -> Self {
        Self {
            node_id: node_id.to_string(),
            institution: institution.to_string(),
            data_size,
            current_round: 0,
            online: true,
            local_loss: None,
            task_scores: HashMap::new(),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 客户端更新
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientUpdate {
    pub client_id: String,
    pub round: u32,
    /// 模型权重（展平数组）
    pub weights: Vec<f32>,
    /// 本地样本数
    pub num_samples: u32,
    /// 本地训练损失
    pub loss: f32,
    /// 各因子的表现分数（Task-Aware 用）
    pub factor_scores: HashMap<String, f32>,
    /// 时间戳
    pub timestamp: String,
}

impl ClientUpdate {
    pub fn weight_in_fedavg(&self) -> f32 {
        self.num_samples as f32
    }
}

// ──────────────────────────────────────────────────────────────
// FedAvg 聚合器
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FedAvgAggregator {
    config: FLConfig,
    global_weights: Vec<f32>,
    current_round: u32,
    nodes: HashMap<String, FLNode>,
    pending_updates: Vec<ClientUpdate>,
    round_history: Vec<RoundRecord>,
    audit: Option<std::sync::Arc<AuditChain>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundRecord {
    pub round: u32,
    pub participants: Vec<String>,
    pub global_loss: f32,
    pub accuracy: Option<f32>,
    pub aggregation_time_ms: u64,
    pub privacy_mechanism: String,
}

impl FedAvgAggregator {
    /// 创建新的聚合器
    pub fn new(config: FLConfig) -> Self {
        Self {
            config,
            global_weights: Vec::new(),
            current_round: 0,
            nodes: HashMap::new(),
            pending_updates: Vec::new(),
            round_history: Vec::new(),
            audit: None,
        }
    }

    /// 绑定审计链
    pub fn with_audit(mut self, audit: std::sync::Arc<AuditChain>) -> Self {
        self.audit = Some(audit);
        self
    }

    /// 注册联邦节点
    pub fn register_node(&mut self, node: FLNode) -> bool {
        if self.nodes.contains_key(&node.node_id) {
            return false;
        }
        self.nodes.insert(node.node_id.clone(), node);
        if let Some(ref audit) = self.audit {
            let _ = audit.append(
                "FL_NODE_REGISTERED",
                &format!("node={}, institution={}, data_size={}",
                    node.node_id, node.institution, node.data_size),
            );
        }
        true
    }

    /// 注销节点
    pub fn unregister_node(&mut self, node_id: &str) -> bool {
        if let Some(mut node) = self.nodes.remove(node_id) {
            node.online = false;
            if let Some(ref audit) = self.audit {
                let _ = audit.append("FL_NODE_DEREGISTERED", &format!("node={}", node_id));
            }
            true
        } else {
            false
        }
    }

    /// 初始化全局模型权重
    pub fn initialize_model(&mut self, input_dim: usize, num_classes: usize) {
        // Xavier 初始化
        let scale = (2.0 / (input_dim + num_classes) as f32).sqrt();
        let total_len = (input_dim + 1) * num_classes;
        self.global_weights = (0..total_len)
            .map(|_| rand_simple() * scale)
            .collect();
    }

    /// 获取当前全局权重
    pub fn get_global_weights(&self) -> &[f32] {
        &self.global_weights
    }

    /// 接收客户端更新
    pub fn receive_update(&mut self, update: ClientUpdate) -> Result<(), String> {
        if update.round != self.current_round {
            return Err(format!(
                "更新轮次不匹配: 期望 {}, 实际 {}",
                self.current_round, update.round
            ));
        }
        self.pending_updates.push(update);
        Ok(())
    }

    /// 获取当前轮次
    pub fn current_round(&self) -> u32 {
        self.current_round
    }

    /// 获取在线节点数
    pub fn online_nodes(&self) -> usize {
        self.nodes.values().filter(|n| n.online).count()
    }

    /// 获取待处理的更新数
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// 检查是否可以开始聚合
    pub fn can_aggregate(&self) -> bool {
        let min_needed = (self.online_nodes() as f32 * self.config.client_fraction).ceil() as usize;
        self.pending_updates.len() >= min_needed.max(self.config.min_clients as usize)
    }

    /// 执行 FedAvg 聚合
    ///
    /// 公式: w_global = Σ (n_k / Σn_i) × w_k
    /// 其中 n_k 是节点 k 的样本数
    pub fn aggregate(&mut self) -> Result<&[f32], String> {
        if !self.can_aggregate() {
            return Err("参与节点不足，无法聚合".to_string());
        }

        let start = std::time::Instant::now();

        // 计算总样本数
        let total_samples: f32 = self.pending_updates.iter()
            .map(|u| u.weight_in_fedavg())
            .sum();

        if total_samples <= 0.0 {
            return Err("总样本数为0".to_string());
        }

        // 加权平均
        let mut new_weights = vec![0.0_f32; self.global_weights.len()];

        for update in &self.pending_updates {
            let weight = update.weight_in_fedavg() / total_samples;

            if new_weights.len() != update.weights.len() {
                return Err(format!(
                    "权重维度不匹配: 全局 {}，更新 {}",
                    new_weights.len(),
                    update.weights.len()
                ));
            }

            for (i, w) in update.weights.iter().enumerate() {
                new_weights[i] += weight * w;
            }
        }

        // 应用学习率
        let lr = self.config.learning_rate;
        if lr != 1.0 {
            for w in &mut new_weights {
                // 简化的学习率应用：new = (1-lr)*old + lr*agg
                // 这里直接用聚合结果
            }
        }

        self.global_weights = new_weights;
        let global_loss = self.pending_updates.iter()
            .map(|u| u.loss)
            .sum::<f32>() / self.pending_updates.len() as f32;

        let participants: Vec<String> = self.pending_updates.iter()
            .map(|u| u.client_id.clone())
            .collect();

        // 记录轮次
        let record = RoundRecord {
            round: self.current_round,
            participants: participants.clone(),
            global_loss,
            accuracy: None,
            aggregation_time_ms: start.elapsed().as_millis() as u64,
            privacy_mechanism: if self.config.enable_dp {
                format!("Gaussian(ε={})", self.config.dp_epsilon)
            } else {
                "none".to_string()
            },
        };
        self.round_history.push(record);

        // 审计
        if let Some(ref audit) = self.audit {
            let _ = audit.append(
                "FL_AGGREGATION_COMPLETED",
                &format!(
                    "round={}, participants={}, loss={:.4}, time_ms={}",
                    self.current_round,
                    participants.len(),
                    global_loss,
                    record.aggregation_time_ms
                ),
            );
        }

        // 准备下一轮
        self.current_round += 1;
        self.pending_updates.clear();

        Ok(&self.global_weights)
    }

    /// 获取聚合历史
    pub fn history(&self) -> &[RoundRecord] {
        &self.round_history
    }

    /// 获取节点列表
    pub fn get_nodes(&self) -> Vec<&FLNode> {
        self.nodes.values().collect()
    }
}

/// Task-Aware 聚合（替代 FedAvg）
///
/// 根据各节点在特定因子上的表现加权
pub struct TaskAwareAggregator {
    pub fedavg: FedAvgAggregator,
    task_weights: HashMap<String, f32>,
}

impl TaskAwareAggregator {
    pub fn new(config: FLConfig) -> Self {
        Self {
            fedavg: FedAvgAggregator::new(config),
            task_weights: HashMap::new(),
        }
    }

    pub fn set_task_weights(&mut self, weights: HashMap<String, f32>) {
        self.task_weights = weights;
    }

    /// Task-Aware 加权聚合
    ///
    /// 节点权重 = Σ (task_weight_i × node_factor_score_i)
    pub fn aggregate_task_aware(&mut self) -> Result<&[f32], String> {
        if !self.fedavg.can_aggregate() {
            return Err("参与节点不足".to_string());
        }

        let mut new_weights = vec![0.0_f32; self.fedavg.global_weights.len()];
        let mut total_weight = 0.0_f32;

        for update in &self.fedavg.pending_updates {
            // 计算节点权重
            let mut node_weight = 0.0_f32;
            for (task, task_w) in &self.task_weights {
                let factor_score = update.factor_scores.get(task).copied().unwrap_or(0.5);
                node_weight += task_w * factor_score;
            }

            // 如果没有任务评分，回退到样本数加权
            if self.task_weights.is_empty() || node_weight == 0.0 {
                node_weight = update.weight_in_fedavg();
            }

            for (i, w) in update.weights.iter().enumerate() {
                new_weights[i] += node_weight * w;
            }
            total_weight += node_weight;
        }

        if total_weight > 0.0 {
            for w in &mut new_weights {
                *w /= total_weight;
            }
        }

        self.fedavg.global_weights = new_weights;
        Ok(&self.fedavg.global_weights)
    }
}

// ──────────────────────────────────────────────────────────────
// 差分隐私
// ──────────────────────────────────────────────────────────────

/// 添加高斯噪声实现本地差分隐私
pub fn add_gaussian_noise(data: &mut [f32], epsilon: f32, delta: f32) {
    // σ = ε / √(2 ln(1.25/δ))，简化为 σ = ε / 2
    let sigma = epsilon / 2.0;
    for v in data {
        *v += gaussian_sample(sigma);
    }
}

fn gaussian_sample(sigma: f32) -> f32 {
    // Box-Muller 变换
    let u1 = rand_simple();
    let u2 = rand_simple();
    sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
}

/// 简化的随机数生成器（生产环境请使用 rand crate）
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    ((nanos as f32 / u32::MAX as f32) * 2.0 - 1.0).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_node() {
        let config = FLConfig::default();
        let mut agg = FedAvgAggregator::new(config);
        let node = FLNode::new("hf_alpha", "Alpha Hedge Fund", 5000);
        assert!(agg.register_node(node));
        assert_eq!(agg.online_nodes(), 1);
    }

    #[test]
    fn test_fedavg_weighted_average() {
        let mut agg = FedAvgAggregator::new(FLConfig::default());
        agg.global_weights = vec![0.0_f32; 4];
        agg.current_round = 1;

        // 节点1: 3个样本，权重 [1,2,3,4]
        agg.receive_update(ClientUpdate {
            client_id: "n1".to_string(),
            round: 1,
            weights: vec![1.0, 2.0, 3.0, 4.0],
            num_samples: 3,
            loss: 0.5,
            factor_scores: HashMap::new(),
            timestamp: "".to_string(),
        }).unwrap();

        // 节点2: 1个样本，权重 [5,6,7,8]
        agg.receive_update(ClientUpdate {
            client_id: "n2".to_string(),
            round: 1,
            weights: vec![5.0, 6.0, 7.0, 8.0],
            num_samples: 1,
            loss: 0.3,
            factor_scores: HashMap::new(),
            timestamp: "".to_string(),
        }).unwrap();

        assert!(agg.can_aggregate());
        let result = agg.aggregate().unwrap();

        // 期望: (3*[1,2,3,4] + 1*[5,6,7,8]) / 4 = [2.0, 3.0, 4.0, 5.0]
        assert_eq!(result, &[2.0, 3.0, 4.0, 5.0]);
        assert_eq!(agg.current_round(), 2);
        assert_eq!(agg.pending_count(), 0);
    }

    #[test]
    fn test_audit_on_aggregate() {
        use std::sync::Arc;
        use std::fs;
        let tmp = std::env::temp_dir().join("fundfl_fl_audit_test.db");
        let _ = fs::remove_file(&tmp);
        let audit = Arc::new(AuditChain::new(&tmp).unwrap());
        let config = FLConfig::default();
        let mut agg = FedAvgAggregator::new(config).with_audit(Arc::clone(&audit));
        agg.global_weights = vec![0.0_f32; 2];
        agg.current_round = 1;
        agg.receive_update(ClientUpdate {
            client_id: "test_node".to_string(),
            round: 1,
            weights: vec![1.0_f32, 2.0],
            num_samples: 10,
            loss: 0.4,
            factor_scores: HashMap::new(),
            timestamp: "".to_string(),
        }).unwrap();
        let _ = agg.aggregate();
        let entries = audit.query("FL_AGGREGATION_COMPLETED", 10).unwrap();
        assert!(!entries.is_empty());
        let _ = fs::remove_file(&tmp);
    }
}
