use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use sha2::{Sha256, Digest};
use std::path::Path;
use std::sync::Mutex;

/// 区块链审计日志
///
/// 每条操作记录通过 SHA-256 哈希链式连接，
/// 确保数据不可篡改。用于合规审计。
pub struct AuditChain {
    conn: Mutex<Connection>,
}

/// 审计条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub index: i64,
    pub timestamp: String,
    pub operation: String,
    pub details: String,
    pub hash: String,
    pub prev_hash: String,
}

impl AuditChain {
    /// 创建新的审计链
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS audit_log (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 timestamp TEXT NOT NULL,
                 operation TEXT NOT NULL,
                 details TEXT NOT NULL DEFAULT '',
                 hash TEXT NOT NULL,
                 prev_hash TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
             CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_log(operation);"
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// 追加一条审计记录
    pub fn append(&self, operation: &str, details: &str) -> Result<AuditEntry> {
        let conn = self.conn.lock().unwrap();

        // 获取上一条的 hash
        let prev_hash: String = conn.query_row(
            "SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        ).unwrap_or_else(|_| "GENESIS".to_string());

        let timestamp = Utc::now().to_rfc3339();

        // 计算哈希: SHA256(index + timestamp + operation + details + prev_hash)
        let index: i64 = conn.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM audit_log",
            [],
            |row| row.get(0),
        )?;

        let mut hasher = Sha256::new();
        hasher.update(index.to_le_bytes());
        hasher.update(timestamp.as_bytes());
        hasher.update(operation.as_bytes());
        hasher.update(details.as_bytes());
        hasher.update(prev_hash.as_bytes());
        let hash = hex::encode(hasher.finalize());

        conn.execute(
            "INSERT INTO audit_log (timestamp, operation, details, hash, prev_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [&timestamp, operation, details, &hash, &prev_hash],
        )?;

        Ok(AuditEntry {
            index,
            timestamp,
            operation: operation.to_string(),
            details: details.to_string(),
            hash,
            prev_hash,
        })
    }

    /// 获取最近的审计记录
    pub fn get_recent(&self, limit: i32, operation_type: Option<&str>) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().unwrap();

        let sql = match operation_type {
            Some(op) => format!(
                "SELECT id, timestamp, operation, details, hash, prev_hash
                 FROM audit_log WHERE operation = '{}' ORDER BY id DESC LIMIT {}",
                op, limit
            ),
            None => format!(
                "SELECT id, timestamp, operation, details, hash, prev_hash
                 FROM audit_log ORDER BY id DESC LIMIT {}",
                limit
            ),
        };

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map([], |row| {
            Ok(AuditEntry {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                operation: row.get(2)?,
                details: row.get(3)?,
                hash: row.get(4)?,
                prev_hash: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// 验证整条哈希链的完整性
    pub fn verify_chain(&self) -> Result<(bool, i64, String)> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log", [], |row| row.get(0)
        )?;

        if count == 0 {
            return Ok((true, 0, String::new()));
        }

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, operation, details, hash, prev_hash
             FROM audit_log ORDER BY id ASC"
        )?;

        let entries: Vec<AuditEntry> = stmt.query_map([], |row| {
            Ok(AuditEntry {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                operation: row.get(2)?,
                details: row.get(3)?,
                hash: row.get(4)?,
                prev_hash: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        let mut prev_hash = "GENESIS".to_string();
        for entry in &entries {
            // 验证 prev_hash 链接
            if entry.prev_hash != prev_hash {
                return Ok((false, count, entry.hash.clone()));
            }

            // 重新计算哈希验证
            let mut hasher = Sha256::new();
            hasher.update(entry.index.to_le_bytes());
            hasher.update(entry.timestamp.as_bytes());
            hasher.update(entry.operation.as_bytes());
            hasher.update(entry.details.as_bytes());
            hasher.update(entry.prev_hash.as_bytes());
            let expected_hash = hex::encode(hasher.finalize());

            if entry.hash != expected_hash {
                return Ok((false, count, entry.hash.clone()));
            }

            prev_hash = entry.hash.clone();
        }

        let latest_hash = entries.last().map(|e| e.hash.clone()).unwrap_or_default();
        Ok((true, count, latest_hash))
    }

    /// 获取链长度
    pub fn chain_length(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log", [], |row| row.get(0)
        )?;
        Ok(count)
    }
}
