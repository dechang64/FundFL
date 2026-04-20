# FundFL — 开源私募基金数据分析与资产定价平台

<div align="center">

**数据不出域，模型共享——让每家私募都能用上全市场的分析能力。**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![PRD](https://img.shields.io/badge/PRD-v1.0-green)](docs/PRD.md)

[English](README.md) | [中文文档](docs/README_zh.md)

</div>

---

## 🎯 一句话介绍

> **输入一只基金代码，返回它的完整风险画像和最相似的5只基金。**

这就是 FundFL MVP 的核心交互。一个 API 调用，两个结果。

## ✨ 核心特性

| 特性 | 说明 |
|------|------|
| 🦀 **Rust 高性能引擎** | HNSW 向量索引，20维特征向量，kNN 搜索 < 1ms |
| 📊 **16项风险指标** | Sharpe, Sortino, Jensen Alpha, Beta, VaR, CVaR, M² 等 |
| 🔍 **相似基金检索** | 基于风险画像的向量搜索，找到最相似的基金 |
| 🔗 **gRPC + REST 双协议** | 程序化调用和前端访问都支持 |
| 🌐 **Web Dashboard** | 开箱即用的可视化面板 |
| 🔐 **区块链审计** | SHA-256 哈希链，确保数据操作不可篡改 |
| 📦 **一键部署** | 单二进制文件，无外部依赖 |

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────┐
│                     用户访问                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │ 浏览器    │  │ curl     │  │ Python   │          │
│  │ Dashboard│  │ REST API │  │ gRPC     │          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘          │
│       │             │             │                 │
│  ─────┴─────────────┴─────────────┴─────────         │
│              Axum (HTTP)  │  Tonic (gRPC)            │
│  ────────────────────────┼──────────────────         │
│                          │                           │
│  ┌───────────┐  ┌────────┴───────┐  ┌───────────┐  │
│  │ FundDB    │  │  VectorDB      │  │ AuditChain│  │
│  │ (SQLite)  │  │  (HNSW/Rust)   │  │ (SHA-256) │  │
│  └───────────┘  └────────────────┘  └───────────┘  │
└─────────────────────────────────────────────────────┘
```

## 🚀 快速开始

```bash
# 1. 克隆
git clone https://github.com/dechang64/fundfl.git
cd fundfl

# 2. 编译运行
cargo run
# gRPC server ready on 0.0.0.0:50051
# REST server ready on 0.0.0.0:8080
# Web dashboard: http://0.0.0.0:8080

# 3. 加载数据（另一个终端）
cd python
pip install -r requirements.txt
python load_data.py
# Loading 63 funds...
# [1/63] PXSGX (小盘股) → Sharpe=0.3645
# [2/63] VFIIX (政府债券) → Sharpe=0.2891
# ...
# Done! 63 funds loaded.

# 4. 使用
# 浏览器打开 http://localhost:8080
# 或 curl http://localhost:8080/api/v1/funds/PXSGX/similar
```

## 📡 API 示例

### REST API

```bash
# 获取基金信息
curl http://localhost:8080/api/v1/funds/PXSGX

# 获取风险指标
curl http://localhost:8080/api/v1/funds/PXSGX/risk

# 搜索相似基金
curl http://localhost:8080/api/v1/funds/PXSGX/similar

# 获取统计
curl http://localhost:8080/api/v1/stats
```

### gRPC (Python)

```python
import grpc
import fundfl_pb2 as pb
import fundfl_pb2_grpc as rpc

channel = grpc.insecure_channel('localhost:50051')
stub = rpc.FundServiceStub(channel)

# 搜索相似基金
resp = stub.SearchSimilar(pb.SearchRequest(
    fund_code="PXSGX",
    k=5
))
for r in resp.results:
    print(f"{r.fund_code}: distance={r.distance:.4f}, sharpe={r.sharpe:.4f}")
```

## 📊 风险指标（16项）

| 指标 | 说明 |
|------|------|
| Sharpe Ratio | 风险调整后收益 |
| Sortino Ratio | 下行风险调整后收益 |
| Jensen's Alpha | 超额收益（CAPM） |
| Beta | 系统性风险暴露 |
| Treynor Ratio | 单位系统性风险收益 |
| Information Ratio | 相对基准的超额收益 |
| Calmar Ratio | 收益/最大回撤 |
| VaR (95%) | 在险价值 |
| CVaR (95%) | 条件在险价值 |
| M² (Modigliani-Modigliani) | 风险等价收益 |
| Max Drawdown | 最大回撤 |
| Skewness | 收益分布偏度 |
| Kurtosis | 收益分布峰度 |
| Win Rate | 正收益月份占比 |
| Annual Return | 年化收益率 |
| Annual Volatility | 年化波动率 |

## 🗺️ 路线图

```
v0.1 (MVP) ──→ v0.2 ──→ v0.3 ──→ v1.0
   │            │         │         │
   │            │         │    联邦学习
   │            │      多因子模型
   │         实时数据采集
   └ 63只基金 + 向量搜索 + Web面板
```

- **v0.1 (当前)**: 63只基金 + HNSW向量搜索 + gRPC/REST + Web面板 + 审计链
- **v0.2 (+2周)**: 天天基金API实时采集 + 500+基金 + Docker部署
- **v0.3 (+3周)**: Fama-French三因子 + 组合优化 + 回测引擎
- **v1.0 (+4周)**: 联邦学习 + Python SDK + 完整文档

## 🔬 技术栈

| 组件 | 技术 |
|------|------|
| 核心引擎 | Rust (axum, tonic, rusqlite) |
| 向量搜索 | HNSW (hnsw crate, Euclidean space) |
| 数据库 | SQLite (WAL mode) |
| 序列化 | Protocol Buffers + JSON |
| 审计 | SHA-256 哈希链 |
| 数据加载 | Python (numpy) |
| 前端 | 原生 HTML/CSS/JS |

## 🤝 与 organoid-fl 的关系

FundFL 复用了 [organoid-fl](https://github.com/dechang64/organoid-fl) 的核心基础设施：

- **HNSW 向量索引** → 从类器官图像特征向量扩展到基金风险特征向量
- **gRPC 服务框架** → 从联邦学习通信扩展到基金查询API
- **区块链审计链** → 从模型训练审计扩展到数据操作审计
- **SQLite 存储** → 从图像元数据扩展到基金元数据

代码复用率约 **60%**。

## 📄 License

Apache-2.0

---

<div align="center">

**FundFL** — 开源的 Wind/Bloomberg 替代方案

</div>
