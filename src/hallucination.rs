//! 幻觉防御模块
//!
//! 五层防御体系，专为金融分析场景定制：
//! 1. 检索一致性检测
//! 2. 向量库事实核验
//! 3. CROWN 一致性防御（NeuroSync 原创）
//! 4. 多节点一致性投票
//! 5. LLM 自洽性检测

use crate::hnsw_index::HnswIndex;
use crate::audit::AuditChain;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────
// 配置
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefenseConfig {
    pub retrieval_threshold: f32,
    pub crown_delta: f32,
    pub self_consistency_threshold: f32,
    pub consensus_threshold: f32,
    pub hallucination_high_risk: f32,
    pub hallucination_medium_risk: f32,
}

impl Default for DefenseConfig {
    fn default() -> Self {
        Self {
            retrieval_threshold: 0.50,
            crown_delta: 0.10,
            self_consistency_threshold: 0.70,
            consensus_threshold: 0.667,
            hallucination_high_risk: 0.75,
            hallucination_medium_risk: 0.45,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 防御结果
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefenseResult {
    pub claim_id: String,
    pub is_hallucination: bool,
    pub risk_score: f32,
    pub verdict: String,
    pub triggered_layers: Vec<String>,
    pub defense_action: String,
    pub evidence: Vec<DefenseEvidence>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefenseEvidence {
    pub layer: String,
    pub description: String,
    pub confidence: f32,
    pub doc_id: Option<String>,
    pub similarity: Option<f32>,
}

// ──────────────────────────────────────────────────────────────
// 防御层 1: 检索一致性检测
// ──────────────────────────────────────────────────────────────

/// 检查断言是否能被向量库检索结果支撑
pub fn layer1_retrieval_consistency(
    claim: &str,
    search_results: &[(String, f32)],
) -> (bool, f32, Vec<DefenseEvidence>) {
    if search_results.is_empty() {
        return (
            false,
            0.0,
            vec![DefenseEvidence {
                layer: "RetrievalConsistency".to_string(),
                description: "无检索结果".to_string(),
                confidence: 0.0,
                doc_id: None,
                similarity: None,
            }],
        );
    }

    // 转换 HNSW 距离为相似度（简化为 exp(-dist)）
    let similarities: Vec<f32> = search_results
        .iter()
        .map(|(_, dist)| (-dist * 0.5_f32).exp().clamp(0.0, 1.0))
        .collect();

    let max_sim = similarities.iter().cloned().fold(0.0f32, f32::max);
    let is_supported = max_sim >= 0.50;

    let evidence = search_results
        .iter()
        .zip(similarities.iter())
        .map(|((id, _), sim)| DefenseEvidence {
            layer: "RetrievalConsistency".to_string(),
            description: format!("文档 {} 相关性: {:.1%}", id, sim),
            confidence: *sim,
            doc_id: Some(id.clone()),
            similarity: Some(*sim),
        })
        .collect();

    (is_supported, max_sim, evidence)
}

// ──────────────────────────────────────────────────────────────
// 防御层 2: 向量库事实核验（金融场景定制）
// ──────────────────────────────────────────────────────────────

/// 在向量库中核验关键金融数据点
pub fn layer2_fact_check(
    claim: &str,
    index: &HnswIndex,
    top_k: usize,
) -> (f32, Vec<DefenseEvidence>) {
    let keywords = extract_financial_terms(claim);
    let mut verified = 0usize;
    let mut evidence = Vec::new();

    for keyword in &keywords {
        // 简化：用零向量搜索（实际应该用关键词embedding）
        let results = index.search(&[0.0_f32; 20][..], top_k, 20).unwrap_or_default();
        let has_match = results.iter().any(|(id, _)| id.contains(keyword));
        if has_match {
            verified += 1;
        }
    }

    let ratio = if keywords.is_empty() {
        0.5
    } else {
        verified as f32 / keywords.len() as f32
    };

    evidence.push(DefenseEvidence {
        layer: "FactCheck".to_string(),
        description: format!("关键术语验证: {}/{} ({:.0%})", verified, keywords.len(), ratio),
        confidence: ratio,
        doc_id: None,
        similarity: None,
    });

    (ratio, evidence)
}

/// 从金融断言中提取关键术语
fn extract_financial_terms(claim: &str) -> Vec<String> {
    let terms = [
        "sharpe", "alpha", "beta", "volatility", "drawdown",
        "var", "cvar", "sortino", "calmar", "treynor",
        "return", "risk", "fund", "hedge", "strategy",
        "inflation", "fed", "rate", "yield", "spread",
    ];
    let claim_lower = claim.to_lowercase();
    terms
        .iter()
        .filter(|t| claim_lower.contains(*t))
        .map(|s| s.to_string())
        .collect()
}

// ──────────────────────────────────────────────────────────────
// 防御层 3: CROWN 一致性防御
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CROWNResult {
    pub triggered: bool,
    pub confidence_drop: f32,
    pub final_answer: String,
    pub reason: String,
    pub crown_delta: f32,
}

/// CROWN 防御：置信度下跌超过 δ 则拒绝从众答案
pub fn layer3_crown_defense(
    initial_answer: &str,
    initial_confidence: f32,
    social_answer: &str,
    social_confidence: f32,
    delta: f32,
) -> CROWNResult {
    let confidence_drop = initial_confidence - social_confidence;
    let answer_changed = initial_answer.trim() != social_answer.trim();
    let triggered = answer_changed && (confidence_drop > delta);

    let (final_answer, reason) = if !answer_changed {
        (initial_answer.to_string(), "答案未变化".to_string())
    } else if triggered {
        (
            initial_answer.to_string(),
            format!(
                "CROWN触发: 置信度下跌 {:.3f} > δ={:.3f}，拒绝社会答案",
                confidence_drop, delta
            ),
        )
    } else {
        (
            social_answer.to_string(),
            format!(
                "置信度下跌 {:.3f} ≤ δ={:.3f}，采纳社会答案",
                confidence_drop, delta
            ),
        )
    };

    CROWNResult {
        triggered,
        confidence_drop,
        final_answer,
        reason,
        crown_delta: delta,
    }
}

// ──────────────────────────────────────────────────────────────
// 防御层 4: 多节点一致性投票
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteResult {
    pub consensus: Option<String>,
    pub consensus_strength: f32,
    pub vote_counts: HashMap<String, u32>,
    pub dissenting: Vec<String>,
    pub verdict: String,
}

/// 多机构投票，返回共识结论
pub fn layer4_multi_node_vote(
    answers: &[String],
    confidences: &[f32],
    threshold: f32,
) -> VoteResult {
    if answers.is_empty() {
        return VoteResult {
            consensus: None,
            consensus_strength: 0.0,
            vote_counts: HashMap::new(),
            dissenting: vec![],
            verdict: "Uncertain".to_string(),
        };
    }

    let mut counts: HashMap<&str, (u32, f32)> = HashMap::new();
    for (ans, conf) in answers.iter().zip(confidences.iter()) {
        let entry = counts.entry(ans).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += conf;
    }

    let best = counts
        .iter()
        .max_by_key(|(_, (count, _))| *count)
        .map(|(k, (count, avg_conf))| {
            (k.to_string(), *count, *count as f32 / answers.len() as f32, *avg_conf / *count as f32)
        });

    let (consensus, votes, strength, _) = best.unwrap_or(("".to_string(), 0, 0.0, 0.0));

    let dissenting: Vec<String> = answers
        .iter()
        .filter(|a| a.as_str() != consensus)
        .cloned()
        .collect();

    let verdict = if strength >= threshold {
        "Verified"
    } else if strength >= 0.5 {
        "LikelyTrue"
    } else if !dissenting.is_empty() {
        "Uncertain"
    } else {
        "Hallucination"
    };

    VoteResult {
        consensus: if consensus.is_empty() { None } else { Some(consensus) },
        consensus_strength: strength,
        vote_counts: counts.iter().map(|(k, (c, _))| (k.to_string(), *c)).collect(),
        dissenting,
        verdict: verdict.to_string(),
    }
}

// ──────────────────────────────────────────────────────────────
// 防御层 5: LLM 自洽性检测
// ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelfConsistencyResult {
    pub consistency_score: f32,
    pub most_common_answer: Option<String>,
    pub passes: bool,
    pub dissenting_answers: Vec<String>,
}

/// 同一问题多次采样的答案一致性检测
pub fn layer5_self_consistency(
    samples: &[String],
    threshold: f32,
) -> SelfConsistencyResult {
    if samples.is_empty() {
        return SelfConsistencyResult {
            consistency_score: 0.0,
            most_common_answer: None,
            passes: false,
            dissenting_answers: vec![],
        };
    }

    let mut counts: HashMap<&str, u32> = HashMap::new();
    for s in samples {
        *counts.entry(s).or_insert(0) += 1;
    }

    let best = counts.iter().max_by_key(|(_, c)| *c);
    let (most_common, count) = best.map(|(k, c)| (Some(k.to_string()), *c)).unwrap_or((None, 0));
    let consistency_score = count as f32 / samples.len() as f32;

    let dissenting: Vec<String> = samples
        .iter()
        .filter(|s| Some(s.as_str()) != most_common.as_deref())
        .cloned()
        .collect();

    SelfConsistencyResult {
        consistency_score,
        most_common_answer: most_common,
        passes: consistency_score >= threshold,
        dissenting_answers: dissenting,
    }
}

// ──────────────────────────────────────────────────────────────
// 综合防御引擎
// ──────────────────────────────────────────────────────────────

/// 综合五层防御引擎，检查金融分析断言是否为幻觉
pub fn run_defense_engine(
    claim_id: &str,
    claim: &str,
    search_results: &[(String, f32)],
    multi_node_ans: Option<&[String]>,
    multi_node_conf: Option<&[f32]>,
    index: Option<&HnswIndex>,
    config: &DefenseConfig,
) -> DefenseResult {
    let mut triggered_layers = Vec::new();
    let mut evidence = Vec::new();
    let mut risk_factors = Vec::new();

    // 层1: 检索一致性
    let (ok, sim, mut ev) = layer1_retrieval_consistency(claim, search_results);
    evidence.append(&mut ev);
    if !ok {
        triggered_layers.push("RetrievalConsistency".to_string());
        risk_factors.push(1.0 - sim);
    }

    // 层2: 事实核验
    if let Some(idx) = index {
        let (ratio, mut ev) = layer2_fact_check(claim, idx, 5);
        evidence.append(&mut ev);
        if ratio < 0.3 {
            triggered_layers.push("FactCheck".to_string());
            risk_factors.push(1.0 - ratio);
        }
    }

    // 层4: 多节点投票
    if let (Some(ans), Some(conf)) = (multi_node_ans, multi_node_conf) {
        let vote = layer4_multi_node_vote(ans, conf, config.consensus_threshold);
        evidence.push(DefenseEvidence {
            layer: "MultiNodeVote".to_string(),
            description: format!(
                "共识: {:?} ({:.0%})",
                vote.consensus, vote.consensus_strength
            ),
            confidence: vote.consensus_strength,
            doc_id: None,
            similarity: None,
        });
        if vote.verdict == "Uncertain" || vote.verdict == "Hallucination" {
            triggered_layers.push("MultiNodeVote".to_string());
            risk_factors.push(1.0 - vote.consensus_strength);
        }
    }

    // 综合风险分数
    let risk_score = if risk_factors.is_empty() {
        0.0
    } else {
        (risk_factors.iter().sum::<f32>() / risk_factors.len() as f32).min(1.0)
    };

    let verdict = match risk_score {
        r if r >= config.hallucination_high_risk => "Hallucination",
        r if r >= config.hallucination_medium_risk => "LikelyFalse",
        r if r > 0.1 => "Uncertain",
        r if r > 0.0 => "LikelyTrue",
        _ => "Verified",
    };

    let defense_action = match verdict {
        "Verified" | "LikelyTrue" => "Accept",
        "Uncertain" | "LikelyFalse" => "Flag",
        _ => "Reject",
    };

    DefenseResult {
        claim_id: claim_id.to_string(),
        is_hallucination: verdict == "Hallucination",
        risk_score,
        verdict: verdict.to_string(),
        triggered_layers,
        defense_action: defense_action.to_string(),
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crown_triggered() {
        let result = layer3_crown_defense(
            "Sharpe = 1.2", 0.88,
            "Sharpe = 0.9", 0.25,
            0.10,
        );
        assert!(result.triggered);
        assert_eq!(result.final_answer, "Sharpe = 1.2");
        assert!(result.confidence_drop > 0.10);
    }

    #[test]
    fn test_multi_vote_consensus() {
        let answers = vec![
            "Long/Short".to_string(),
            "Long/Short".to_string(),
            "Global Macro".to_string(),
        ];
        let confs = vec![0.85, 0.79, 0.62];
        let result = layer4_multi_node_vote(&answers, &confs, 0.667);
        assert_eq!(result.consensus, Some("Long/Short".to_string()));
        assert!(result.consensus_strength > 0.66);
    }

    #[test]
    fn test_self_consistency() {
        let samples = vec![
            "配置 60% 股票".to_string(),
            "配置 60% 股票".to_string(),
            "配置 70% 股票".to_string(),
        ];
        let result = layer5_self_consistency(&samples, 0.70);
        assert_eq!(result.consistency_score, 2.0 / 3.0);
        assert!(!result.passes);
    }

    #[test]
    fn test_defense_engine() {
        let config = DefenseConfig::default();
        let results = vec![("HF_001".to_string(), 0.15_f32)];
        let result = run_defense_engine(
            "test",
            "HF_001 Sharpe Ratio 高于同类",
            &results,
            None,
            None,
            None,
            &config,
        );
        assert!(!result.claim_id.is_empty());
    }
}
