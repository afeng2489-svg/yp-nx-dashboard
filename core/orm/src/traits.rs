/// 通用 Repository trait — 封装持久化 CRUD 操作。
///
/// 具体实现处理存储细节（SQLite、Postgres、in-memory 测试等），
/// 业务逻辑依赖 trait 而非具体实现。
pub trait Repository<T, K> {
    type Error;

    /// 保存实体（插入或更新）
    fn save(&self, entity: &T) -> Result<(), Self::Error>;

    /// 按主键查找
    fn find(&self, id: &K) -> Result<Option<T>, Self::Error>;

    /// 删除实体，返回是否实际删除了行
    fn delete(&self, id: &K) -> Result<bool, Self::Error>;

    /// 列出实体，可选限制数量
    fn list(&self, limit: Option<usize>) -> Result<Vec<T>, Self::Error>;

    /// 总数
    fn count(&self) -> Result<usize, Self::Error>;
}
