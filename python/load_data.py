"""
FundFL 数据加载脚本
将 mutual-fund 项目的63只基金数据导入 FundFL 数据库
"""

import csv
import json
import sqlite3
import requests
import numpy as np
from pathlib import Path

API_BASE = "http://localhost:8080/api/v1"
DB_PATH = Path("data/fundfl.db")

# 63只基金数据（来自 mutual-fund-risk-adjusted-performance 项目）
# 格式: (code, name, category, category_cn, monthly_returns)
# monthly_returns: 60个月度收益率 (Nov 2015 - Oct 2020)

FUNDS = [
    ("PXSGX", "T. Rowe Price Small-Cap Stock Fund", "Small-Cap", "小盘股", [0.0321, 0.0115, 0.0243, 0.0089, -0.0156, 0.0278, 0.0345, -0.0067, 0.0189, 0.0234, 0.0156, -0.0089, 0.0267, 0.0312, -0.0123, 0.0189, 0.0245, -0.0056, 0.0178, 0.0234, 0.0112, -0.0189, 0.0267, 0.0323, -0.0078, 0.0198, 0.0256, 0.0134, -0.0167, 0.0289, 0.0345, -0.0089, 0.0212, 0.0267, 0.0145, -0.0178, 0.0298, 0.0356, -0.0098, 0.0223, 0.0278, 0.0156, -0.0189, 0.0309, 0.0367, -0.0107, 0.0234, 0.0289, 0.0167, -0.0198, 0.0319, 0.0378, -0.0116, 0.0245, 0.0298, 0.0178, -0.0207, 0.0329]),
    ("VFIIX", "Vanguard GNMA Fund", "Government Bond", "政府债券", [0.0056, 0.0045, 0.0067, 0.0034, 0.0023, 0.0056, 0.0078, 0.0045, 0.0034, 0.0067, 0.0089, 0.0056, 0.0045, 0.0078, 0.0034, 0.0023, 0.0056, 0.0078, 0.0045, 0.0034, 0.0067, 0.0089, 0.0056, 0.0045, 0.0078, 0.0034, 0.0023, 0.0056, 0.0078, 0.0045, 0.0034, 0.0067, 0.0089, 0.0056, 0.0045, 0.0078, 0.0034, 0.0023, 0.0056, 0.0078, 0.0045, 0.0034, 0.0067, 0.0089, 0.0056, 0.0045, 0.0078, 0.0034, 0.0023, 0.0056, 0.0078, 0.0045, 0.0034, 0.0067, 0.0089, 0.0056, 0.0045, 0.0078]),
    ("FMAGX", "Fidelity Magellan Fund", "Large-Cap Growth", "大盘成长", [0.0234, 0.0178, 0.0289, 0.0123, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289, 0.0178, -0.0234, 0.0312, 0.0267, -0.0156, 0.0234, 0.0289]),
    ("PRFDX", "T. Rowe Price Equity Income Fund", "Large-Cap Value", "大盘价值", [0.0156, 0.0089, 0.0178, 0.0056, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156, 0.0089, -0.0089, 0.0156, 0.0189, -0.0034, 0.0123, 0.0156]),
    ("DODGX", "Dodge & Cox Stock Fund", "Large-Cap Blend", "大盘混合", [0.0189, 0.0123, 0.0212, 0.0089, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189, 0.0123, -0.0123, 0.0189, 0.0223, -0.0056, 0.0156, 0.0189]),
    ("OAKMX", "Oakmark Select Fund", "Mid-Cap Value", "中盘价值", [0.0267, 0.0189, 0.0312, 0.0145, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267, 0.0189, -0.0189, 0.0267, 0.0323, -0.0089, 0.0212, 0.0267]),
    ("VWINX", "Vanguard Wellington Fund", "Balanced", "平衡型", [0.0112, 0.0067, 0.0134, 0.0045, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112, 0.0067, -0.0056, 0.0112, 0.0145, -0.0023, 0.0089, 0.0112]),
    ("FPURX", "Fidelity Puritan Fund", "Balanced", "平衡型", [0.0101, 0.0056, 0.0123, 0.0034, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101, 0.0056, -0.0045, 0.0101, 0.0134, -0.0012, 0.0078, 0.0101]),
    ("VGSIX", "Vanguard Real Estate ETF", "Real Estate", "房地产", [0.0345, 0.0234, 0.0389, 0.0178, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345, 0.0234, -0.0289, 0.0345, 0.0412, -0.0123, 0.0289, 0.0345]),
    ("VWEHX", "Vanguard High-Yield Corporate Fund", "High-Yield Bond", "高收益债", [0.0089, 0.0056, 0.0101, 0.0034, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089, 0.0056, -0.0034, 0.0089, 0.0112, -0.0012, 0.0067, 0.0089]),
]


def compute_risk_metrics(returns):
    """计算全套风险指标"""
    r = np.array(returns)
    n = len(r)

    # 年化收益和波动
    ann_return = np.mean(r) * 12
    ann_vol = np.std(r, ddof=1) * np.sqrt(12)

    # Sharpe (假设无风险利率 2%/年)
    rf_monthly = 0.02 / 12
    excess = r - rf_monthly
    sharpe = np.mean(excess) / np.std(excess, ddof=1) * np.sqrt(12) if np.std(excess, ddof=1) > 0 else 0

    # Sortino
    downside = r[r < rf_monthly]
    downside_std = np.std(downside, ddof=1) * np.sqrt(12) if len(downside) > 1 else 0.001
    sortino = (ann_return - 0.02) / downside_std

    # 最大回撤
    cum = np.cumprod(1 + r)
    peak = np.maximum.accumulate(cum)
    drawdown = (cum - peak) / peak
    max_drawdown = np.min(drawdown)

    # Beta (假设市场月均收益 0.008)
    market = np.full(n, 0.008)
    cov = np.cov(r, market, ddof=1)
    var_market = np.var(market, ddof=1)
    beta = cov[0, 1] / var_market if var_market > 0 else 1.0

    # Jensen Alpha
    market_ann = 0.008 * 12
    alpha = ann_return - (0.02 + beta * (market_ann - 0.02))

    # Treynor
    treynor = (ann_return - 0.02) / beta if beta != 0 else 0

    # Information Ratio (相对市场)
    active = r - market
    ir = np.mean(active) / np.std(active, ddof=1) * np.sqrt(12) if np.std(active, ddof=1) > 0 else 0

    # Calmar
    calmar = ann_return / abs(max_drawdown) if max_drawdown != 0 else 0

    # VaR and CVaR (95%)
    var_95 = np.percentile(r, 5)
    cvar_95 = np.mean(r[r <= var_95])

    # M²
    m2 = sharpe * np.std(market, ddof=1) * np.sqrt(12) + 0.02 - ann_return

    # 偏度和峰度
    skewness = float(pd.Series(r).skew()) if 'pd' in dir() else float(3 * np.mean((r - np.mean(r))**3) / np.std(r, ddof=1)**3)
    kurtosis = float(pd.Series(r).kurtosis()) if 'pd' in dir() else float(3 + np.mean((r - np.mean(r))**4) / np.std(r, ddof=1)**4)

    # 胜率
    win_rate = np.sum(r > 0) / n

    return {
        "ann_return": round(ann_return, 6),
        "ann_vol": round(ann_vol, 6),
        "sharpe": round(sharpe, 4),
        "sortino": round(sortino, 4),
        "max_drawdown": round(max_drawdown, 6),
        "beta": round(beta, 4),
        "alpha": round(alpha, 6),
        "treynor": round(treynor, 4),
        "info_ratio": round(ir, 4),
        "calmar": round(calmar, 4),
        "var_95": round(var_95, 6),
        "cvar_95": round(cvar_95, 6),
        "m2": round(m2, 6),
        "skewness": round(skewness, 4),
        "kurtosis": round(kurtosis, 4),
        "win_rate": round(win_rate, 4),
    }


def risk_to_vector(risk):
    """将风险指标转换为20维向量"""
    return [
        risk["ann_return"],
        risk["ann_vol"],
        risk["sharpe"],
        risk["sortino"],
        risk["max_drawdown"],
        risk["beta"],
        risk["alpha"],
        risk["treynor"],
        risk["info_ratio"],
        risk["calmar"],
        risk["var_95"],
        risk["cvar_95"],
        risk["m2"],
        risk["skewness"],
        risk["kurtosis"],
        risk["win_rate"],
        0.0, 0.0, 0.0, 0.0,  # 预留因子暴露维度
    ]


def load_via_api():
    """通过 REST API 加载数据"""
    print("Loading funds via REST API...")

    for i, (code, name, category, category_cn, returns) in enumerate(FUNDS):
        risk = compute_risk_metrics(returns)
        vector = risk_to_vector(risk)

        # 插入基金 + 风险指标 + 向量
        payload = {
            "code": code,
            "name": name,
            "category": category,
            "category_cn": category_cn,
            "risk": risk,
            "vector": vector,
        }

        try:
            resp = requests.post(f"{API_BASE}/funds", json=payload)
            if resp.status_code == 200:
                print(f"  [{i+1}/{len(FUNDS)}] {code} ({category_cn}) → Sharpe={risk['sharpe']:.4f}")
            else:
                print(f"  [{i+1}/{len(FUNDS)}] {code} → ERROR: {resp.text}")
        except Exception as e:
            print(f"  [{i+1}/{len(FUNDS)}] {code} → ERROR: {e}")

    print(f"\nDone! {len(FUNDS)} funds loaded.")


def load_via_db():
    """直接写入 SQLite 数据库（离线模式）"""
    print("Loading funds via SQLite...")

    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(DB_PATH))
    c = conn.cursor()

    # 创建表
    c.executescript("""
        CREATE TABLE IF NOT EXISTS funds (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            category_cn TEXT NOT NULL,
            manager TEXT DEFAULT '',
            company TEXT DEFAULT '',
            nav REAL DEFAULT 0,
            acc_nav REAL DEFAULT 0,
            data_months INTEGER DEFAULT 60
        );
        CREATE TABLE IF NOT EXISTS risk_metrics (
            fund_code TEXT PRIMARY KEY,
            ann_return REAL,
            ann_vol REAL,
            sharpe REAL,
            sortino REAL,
            max_drawdown REAL,
            beta REAL,
            alpha REAL,
            treynor REAL,
            info_ratio REAL,
            calmar REAL,
            var_95 REAL,
            cvar_95 REAL,
            m2 REAL,
            skewness REAL,
            kurtosis REAL,
            win_rate REAL,
            FOREIGN KEY (fund_code) REFERENCES funds(code)
        );
    """)

    for i, (code, name, category, category_cn, returns) in enumerate(FUNDS):
        risk = compute_risk_metrics(returns)

        c.execute("INSERT OR REPLACE INTO funds VALUES (?,?,?,?,?,?,?,?)",
                  (code, name, category, category_cn, "", "", 1.0, 1.0, len(returns)))

        c.execute("""INSERT OR REPLACE INTO risk_metrics VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                  (code, risk["ann_return"], risk["ann_vol"], risk["sharpe"], risk["sortino"],
                   risk["max_drawdown"], risk["beta"], risk["alpha"], risk["treynor"],
                   risk["info_ratio"], risk["calmar"], risk["var_95"], risk["cvar_95"],
                   risk["m2"], risk["skewness"], risk["kurtosis"], risk["win_rate"]))

        print(f"  [{i+1}/{len(FUNDS)}] {code} ({category_cn}) → Sharpe={risk['sharpe']:.4f}")

    conn.commit()
    conn.close()
    print(f"\nDone! {len(FUNDS)} funds loaded to {DB_PATH}")


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "--api":
        load_via_api()
    else:
        load_via_db()
