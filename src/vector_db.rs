use anyhow::Result;
use crate::hnsw_index::HnswIndex;
use std::collections::HashMap;

/// 向量数据库：管理基金特征向量的存储和检索
pub struct VectorDb {
    index: HnswIndex,
    /// id -> 向量的映射（用于重建索引）
    vectors: HashMap<String, Vec<f32>>,
    /// id -> 元数据
    metadata: HashMap<String, HashMap<String, String>>,
}

impl VectorDb {
    /// 创建新的向量数据库
    pub fn new(dimension: usize) -> Self {
        Self {
            index: HnswIndex::with_defaults(dimension),
            vectors: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// 插入一个向量及其元数据
    pub fn insert(&mut self, id: &str, vector: &[f32], meta: Option<HashMap<String, String>>) -> Result<()> {
        if self.vectors.contains_key(id) {
            // 更新：需要重建索引
            self.vectors.insert(id.to_string(), vector.to_vec());
            if let Some(m) = meta {
                self.metadata.insert(id.to_string(), m);
            }
            self.rebuild_index()?;
        } else {
            self.index.insert(id, vector)?;
            self.vectors.insert(id.to_string(), vector.to_vec());
            if let Some(m) = meta {
                self.metadata.insert(id.to_string(), m);
            }
        }
        Ok(())
    }

    /// 批量插入
    pub fn insert_batch(
        &mut self,
        entries: Vec<(String, Vec<f32>, Option<HashMap<String, String>>)>,
    ) -> Result<usize> {
        for (id, vector, meta) in &entries {
            self.index.insert(id, vector)?;
            self.vectors.insert(id.clone(), vector.clone());
            if let Some(m) = meta {
                self.metadata.insert(id.clone(), m.clone());
            }
        }
        Ok(entries.len())
    }

    /// 搜索相似向量
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        let ef_search = std::cmp::max(k * 4, 50);
        let raw = self.index.search(query, k, ef_search)?;
        let results: Vec<SearchResult> = raw
            .into_iter()
            .map(|(id, distance)| {
                let meta = self.metadata.get(&id).cloned().unwrap_or_default();
                SearchResult { id, distance, meta }
            })
            .collect();
        Ok(results)
    }

    /// 获取向量
    pub fn get(&self, id: &str) -> Option<&Vec<f32>> {
        self.vectors.get(id)
    }

    /// 获取元数据
    pub fn get_metadata(&self, id: &str) -> Option<&HashMap<String, String>> {
        self.metadata.get(id)
    }

    /// 向量数量
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// 重建索引（更新或删除后调用）
    fn rebuild_index(&mut self) -> Result<()> {
        let dimension = self.index.dimension();
        let mut new_index = HnswIndex::with_defaults(dimension);
        for (id, vector) in &self.vectors {
            new_index.insert(id, vector)?;
        }
        self.index = new_index;
        Ok(())
    }
}

/// 向量搜索结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub distance: f32,
    pub meta: HashMap<String, String>,
}
