use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

/// 基金元数据存储（SQLite）
pub struct FundDb {
    conn: Mutex<Connection>,
}

/// 基金基本信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Fund {
    pub code: String,
    pub name: String,
    pub category: String,
    pub category_cn: String,
    pub manager: String,
    pub company: String,
    pub nav: f64,
    pub acc_nav: f64,
    pub data_months: i32,
}

/// 风险指标
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RiskMetrics {
    pub fund_code: String,
    pub ann_return: f64,
    pub ann_vol: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub beta: f64,
    pub alpha: f64,
    pub treynor: f64,
    pub info_ratio: f64,
    pub calmar: f64,
    pub var_95: f64,
    pub cvar_95: f64,
    pub m2: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub win_rate: f64,
}

/// 类别统计
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub category_cn: String,
    pub count: i64,
    pub avg_sharpe: f64,
    pub avg_alpha: f64,
}

impl FundDb {
    /// 初始化数据库，创建表结构
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;"
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS funds (
                code TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                category_cn TEXT NOT NULL DEFAULT '',
                manager TEXT NOT NULL DEFAULT '',
                company TEXT NOT NULL DEFAULT '',
                nav REAL NOT NULL DEFAULT 0.0,
                acc_nav REAL NOT NULL DEFAULT 0.0,
                data_months INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS risk_metrics (
                fund_code TEXT PRIMARY KEY REFERENCES funds(code),
                ann_return REAL NOT NULL DEFAULT 0.0,
                ann_vol REAL NOT NULL DEFAULT 0.0,
                sharpe REAL NOT NULL DEFAULT 0.0,
                sortino REAL NOT NULL DEFAULT 0.0,
                max_drawdown REAL NOT NULL DEFAULT 0.0,
                beta REAL NOT NULL DEFAULT 0.0,
                alpha REAL NOT NULL DEFAULT 0.0,
                treynor REAL NOT NULL DEFAULT 0.0,
                info_ratio REAL NOT NULL DEFAULT 0.0,
                calmar REAL NOT NULL DEFAULT 0.0,
                var_95 REAL NOT NULL DEFAULT 0.0,
                cvar_95 REAL NOT NULL DEFAULT 0.0,
                m2 REAL NOT NULL DEFAULT 0.0,
                skewness REAL NOT NULL DEFAULT 0.0,
                kurtosis REAL NOT NULL DEFAULT 0.0,
                win_rate REAL NOT NULL DEFAULT 0.0
            );

            CREATE INDEX IF NOT EXISTS idx_funds_category ON funds(category);
            CREATE INDEX IF NOT EXISTS idx_risk_sharpe ON risk_metrics(sharpe);
            CREATE INDEX IF NOT EXISTS idx_risk_alpha ON risk_metrics(alpha);
            "
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 插入基金信息
    pub fn insert_fund(&self, fund: &Fund) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO funds (code, name, category, category_cn, manager, company, nav, acc_nav, data_months)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                fund.code, fund.name, fund.category, fund.category_cn,
                fund.manager, fund.company, fund.nav, fund.acc_nav, fund.data_months
            ],
        )?;
        Ok(())
    }

    /// 插入风险指标
    pub fn insert_risk_metrics(&self, risk: &RiskMetrics) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO risk_metrics
             (fund_code, ann_return, ann_vol, sharpe, sortino, max_drawdown,
              beta, alpha, treynor, info_ratio, calmar, var_95, cvar_95,
              m2, skewness, kurtosis, win_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                risk.fund_code, risk.ann_return, risk.ann_vol, risk.sharpe,
                risk.sortino, risk.max_drawdown, risk.beta, risk.alpha,
                risk.treynor, risk.info_ratio, risk.calmar, risk.var_95,
                risk.cvar_95, risk.m2, risk.skewness, risk.kurtosis, risk.win_rate
            ],
        )?;
        Ok(())
    }

    /// 查询基金信息
    pub fn get_fund(&self, code: &str) -> Result<Option<Fund>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT code, name, category, category_cn, manager, company, nav, acc_nav, data_months
             FROM funds WHERE code = ?1"
        )?;
        let fund = stmt.query_row(params![code], |row| {
            Ok(Fund {
                code: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                category_cn: row.get(3)?,
                manager: row.get(4)?,
                company: row.get(5)?,
                nav: row.get(6)?,
                acc_nav: row.get(7)?,
                data_months: row.get(8)?,
            })
        }).ok();
        Ok(fund)
    }

    /// 查询风险指标
    pub fn get_risk_metrics(&self, code: &str) -> Result<Option<RiskMetrics>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fund_code, ann_return, ann_vol, sharpe, sortino, max_drawdown,
                    beta, alpha, treynor, info_ratio, calmar, var_95, cvar_95,
                    m2, skewness, kurtosis, win_rate
             FROM risk_metrics WHERE fund_code = ?1"
        )?;
        let risk = stmt.query_row(params![code], |row| {
            Ok(RiskMetrics {
                fund_code: row.get(0)?,
                ann_return: row.get(1)?,
                ann_vol: row.get(2)?,
                sharpe: row.get(3)?,
                sortino: row.get(4)?,
                max_drawdown: row.get(5)?,
                beta: row.get(6)?,
                alpha: row.get(7)?,
                treynor: row.get(8)?,
                info_ratio: row.get(9)?,
                calmar: row.get(10)?,
                var_95: row.get(11)?,
                cvar_95: row.get(12)?,
                m2: row.get(13)?,
                skewness: row.get(14)?,
                kurtosis: row.get(15)?,
                win_rate: row.get(16)?,
            })
        }).ok();
        Ok(risk)
    }

    /// 列表查询（支持分页/排序/筛选）
    pub fn list_funds(
        &self,
        category: Option<&str>,
        sort_by: &str,
        sort_order: &str,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<Fund>, i64)> {
        let conn = self.conn.lock().unwrap();

        let mut where_clause = String::new();
        if let Some(cat) = category {
            where_clause = format!("WHERE f.category = '{}'", cat);
        }

        let order_col = match sort_by {
            "sharpe" => "r.sharpe",
            "alpha" => "r.alpha",
            "return" => "r.ann_return",
            "drawdown" => "r.max_drawdown",
            "name" => "f.name",
            _ => "f.code",
        };
        let order_dir = if sort_order == "asc" { "ASC" } else { "DESC" };

        let count_sql = format!(
            "SELECT COUNT(*) FROM funds f LEFT JOIN risk_metrics r ON f.code = r.fund_code {}",
            where_clause
        );
        let total: i64 = conn.query_row(&count_sql, [], |row| row.get(0))?;

        let offset = (page - 1) * page_size;
        let sql = format!(
            "SELECT f.code, f.name, f.category, f.category_cn, f.manager, f.company, f.nav, f.acc_nav, f.data_months
             FROM funds f
             LEFT JOIN risk_metrics r ON f.code = r.fund_code
             {}
             ORDER BY {} {}
             LIMIT {} OFFSET {}",
            where_clause, order_col, order_dir, page_size, offset
        );

        let mut stmt = conn.prepare(&sql)?;
        let funds = stmt.query_map([], |row| {
            Ok(Fund {
                code: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                category_cn: row.get(3)?,
                manager: row.get(4)?,
                company: row.get(5)?,
                nav: row.get(6)?,
                acc_nav: row.get(7)?,
                data_months: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok((funds, total))
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> Result<(i64, Vec<CategoryStats>)> {
        let conn = self.conn.lock().unwrap();

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM funds", [], |row| row.get(0)
        )?;

        let mut stmt = conn.prepare(
            "SELECT f.category, f.category_cn, COUNT(*) as cnt,
                    COALESCE(AVG(r.sharpe), 0) as avg_sharpe,
                    COALESCE(AVG(r.alpha), 0) as avg_alpha
             FROM funds f
             LEFT JOIN risk_metrics r ON f.code = r.fund_code
             GROUP BY f.category
             ORDER BY cnt DESC"
        )?;

        let categories = stmt.query_map([], |row| {
            Ok(CategoryStats {
                category: row.get(0)?,
                category_cn: row.get(1)?,
                count: row.get(2)?,
                avg_sharpe: row.get(3)?,
                avg_alpha: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok((total, categories))
    }
}
