<div align="center">

# FundFL v2

### 开源私募基金风险分析与资产定价平台

**数据不出域，模型共享——让每家私募都能用上全市场的分析能力。**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.10+-blue?logo=python)](https://python.org/)
[![PyTorch](https://img.shields.io/badge/PyTorch-2.0+-ee4c2c?logo=pytorch)](https://pytorch.org/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

</div>

---

## 🎯 一句话介绍

> **输入一只基金代码，返回它的完整风险画像和最相似的5只基金。**

## 🆕 v2 Features

| Feature | v1 | v2 |
|---------|----|----|
| Risk Metrics | 16项 | **16项** (Sharpe, Sortino, Jensen Alpha, Beta, VaR, CVaR, M²...) |
| Feature Vector | 20-dim | **16-dim** risk profile vector |
| Similarity | HNSW kNN | **Cosine similarity** + HNSW |
| Python SDK | load_data.py | **RiskAnalyzer** class with full API |
| Benchmark | — | **Jensen Alpha, Beta, Tracking Error, IR** |

## 📊 16 Risk Metrics

| # | Metric | Description |
|---|--------|-------------|
| 1 | Annualized Return | 年化收益率 |
| 2 | Annualized Volatility | 年化波动率 |
| 3 | Sharpe Ratio | 风险调整收益 |
| 4 | Sortino Ratio | 下行风险调整收益 |
| 5 | Jensen's Alpha | 超额收益（相对基准） |
| 6 | Beta | 系统性风险暴露 |
| 7 | Max Drawdown | 最大回撤 |
| 8 | Calmar Ratio | 回撤调整收益 |
| 9 | VaR (95%) | 在险价值 |
| 10 | CVaR (95%) | 条件在险价值 |
| 11 | Information Ratio | 信息比率 |
| 12 | Tracking Error | 跟踪误差 |
| 13 | Skewness | 收益偏度 |
| 14 | Kurtosis | 收益峰度 |
| 15 | M² | Modigliani-Modigliani |
| 16 | Win Rate | 胜率 |

## 🚀 Quick Start

```bash
cd python && pip install -r requirements.txt
python -c "
from analysis import RiskAnalyzer
import numpy as np
analyzer = RiskAnalyzer()
returns = np.random.normal(0.001, 0.02, 252)
profile = analyzer.compute(returns, fund_code='000001', fund_name='Test')
print(f'Sharpe: {profile.sharpe_ratio:.3f}, MaxDD: {profile.max_drawdown:.3f}')
"
```

## 📊 Tests

```bash
cd python && python -m pytest tests/ -v
# 6 passed
```

## 🤝 Related Projects

| Project | Domain | Shared Infra |
|---------|--------|-------------|
| [organoid-fl](https://github.com/dechang64/organoid-fl) | Medical imaging | HNSW, gRPC, audit |
| [embodied-fl](https://github.com/dechang64/embodied-fl) | Robotics | HNSW, gRPC, audit |
| [defect-fl](https://github.com/dechang64/defect-fl) | PCB inspection | HNSW, gRPC, audit |
| [Reading-FL](https://github.com/dechang64/reading-fl) | Reading | HNSW, audit |

## 📄 License

Apache-2.0

---

<div align="center">

**FundFL v2** — 开源的 Wind/Bloomberg 替代方案

</div>
