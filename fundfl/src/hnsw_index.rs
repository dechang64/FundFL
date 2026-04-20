use anyhow::Result;
use std::path::Path;

/// HNSW 向量索引（基于 hnsw crate）
///
/// 支持泛型维度，用于基金特征向量的快速近似最近邻搜索。
/// 20维向量，63只基金，搜索延迟 < 1ms。
pub struct HnswIndex {
    index: hnsw::Hnsw<f32, space::Euclidean>,
    dimension: usize,
    ids: Vec<String>,
}

impl HnswIndex {
    /// 创建新的 HNSW 索引
    ///
    /// # 参数
    /// - `dimension`: 向量维度（MVP 使用 20 维）
    /// - `max_elements`: 最大元素数量
    /// - `ef_construction`: 构建时搜索宽度（越大越精确，越慢）
    /// - `m`: 每层最大连接数
    pub fn new(dimension: usize, max_elements: usize, ef_construction: usize, m: usize) -> Self {
        let config = hnsw::Config {
            max_elements,
            ef_construction,
            m,
            ..Default::default()
        };
        let index = hnsw::Hnsw::new(config);
        Self {
            index,
            dimension,
            ids: Vec::with_capacity(max_elements),
        }
    }

    /// 使用默认参数创建索引（适合 MVP 场景）
    pub fn with_defaults(dimension: usize) -> Self {
        Self::new(dimension, 10000, 200, 16)
    }

    /// 插入一个向量
    pub fn insert(&mut self, id: &str, vector: &[f32]) -> Result<()> {
        assert_eq!(vector.len(), self.dimension, "Vector dimension mismatch");
        self.ids.push(id.to_string());
        self.index.insert(vector.to_vec());
        Ok(())
    }

    /// 搜索最近的 k 个邻居
    ///
    /// 返回 (id, distance) 列表，按距离升序排列
    pub fn search(&self, query: &[f32], k: usize, ef_search: usize) -> Result<Vec<(String, f32)>> {
        assert_eq!(query.len(), self.dimension, "Query dimension mismatch");
        let neighbors = self.index.search(query, ef_search, k);
        let results: Vec<(String, f32)> = neighbors
            .into_iter()
            .filter_map(|(idx, dist)| {
                self.ids.get(idx).map(|id| (id.clone(), dist))
            })
            .collect();
        Ok(results)
    }

    /// 获取当前索引中的向量数量
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// 从文件加载索引（预留接口）
    pub fn load(_path: &Path) -> Result<Self> {
        // TODO: 实现索引持久化
        anyhow::bail!("Index persistence not yet implemented")
    }

    /// 保存索引到文件（预留接口）
    pub fn save(&self, _path: &Path) -> Result<()> {
        // TODO: 实现索引持久化
        anyhow::bail!("Index persistence not yet implemented")
    }
}
