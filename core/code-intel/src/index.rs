//! 用于快速符号查找的代码索引

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{Symbol, Reference, ReferenceFinder};

/// 索引中的文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    /// 文件路径
    pub path: PathBuf,
    /// 内容哈希
    pub content_hash: u64,
    /// 文件中的符号
    pub symbols: Vec<Symbol>,
    /// 最后修改时间
    pub last_modified: std::time::SystemTime,
}

/// 项目的代码索引
pub struct CodeIndex {
    /// 文件索引
    files: RwLock<HashMap<PathBuf, IndexedFile>>,
    /// 符号索引
    symbol_index: RwLock<HashMap<String, Vec<SymbolLocation>>>,
    /// 项目根目录
    project_root: PathBuf,
}

/// 符号位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolLocation {
    /// 文件路径
    pub file: PathBuf,
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
}

impl CodeIndex {
    /// 创建新的代码索引
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
            symbol_index: RwLock::new(HashMap::new()),
            project_root,
        }
    }

    /// 为文件建立索引
    pub fn index_file(&self, path: PathBuf, symbols: Vec<Symbol>) {
        let file = IndexedFile {
            path: path.clone(),
            content_hash: 0, // 实际应计算哈希
            symbols: symbols.clone(),
            last_modified: std::time::SystemTime::now(),
        };

        // 更新文件索引
        {
            let mut files = self.files.write();
            files.insert(path.clone(), file);
        }

        // 更新符号索引
        {
            let mut symbol_index = self.symbol_index.write();
            for symbol in symbols {
                let location = SymbolLocation {
                    file: path.clone(),
                    line: symbol.line,
                    column: symbol.column,
                };
                symbol_index
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(location);
            }
        }
    }

    /// 从索引中移除文件
    pub fn remove_file(&self, path: &PathBuf) {
        let mut files = self.files.write();
        if let Some(removed) = files.remove(path) {
            // 从符号索引中移除
            let mut symbol_index = self.symbol_index.write();
            for symbol in removed.symbols {
                if let Some(locations) = symbol_index.get_mut(&symbol.name) {
                    locations.retain(|loc| &loc.file != path);
                }
            }
        }
    }

    /// 查找符号的所有位置
    pub fn find_symbol(&self, name: &str) -> Vec<SymbolLocation> {
        let symbol_index = self.symbol_index.read();
        symbol_index.get(name).cloned().unwrap_or_default()
    }

    /// 获取所有已索引的文件
    pub fn get_files(&self) -> Vec<PathBuf> {
        let files = self.files.read();
        files.keys().cloned().collect()
    }

    /// 获取文件中的符号
    pub fn get_file_symbols(&self, path: &PathBuf) -> Option<Vec<Symbol>> {
        let files = self.files.read();
        files.get(path).map(|f| f.symbols.clone())
    }

    /// 按名称模式搜索符号
    pub fn search_symbols(&self, pattern: &str) -> Vec<Symbol> {
        let files = self.files.read();
        let mut results = Vec::new();

        for file in files.values() {
            for symbol in &file.symbols {
                if symbol.name.contains(pattern) {
                    results.push(symbol.clone());
                }
            }
        }

        results
    }

    /// 获取索引统计信息
    pub fn stats(&self) -> IndexStats {
        let files = self.files.read();
        let symbol_index = self.symbol_index.read();

        IndexStats {
            file_count: files.len(),
            symbol_count: symbol_index.values().map(|v| v.len()).sum(),
            unique_symbols: symbol_index.len(),
        }
    }
}

/// 索引统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// 文件数量
    pub file_count: usize,
    /// 符号数量
    pub symbol_count: usize,
    /// 唯一符号数量
    pub unique_symbols: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_operations() {
        let index = CodeIndex::new(PathBuf::from("/test"));

        let symbols = vec![
            Symbol {
                name: "test_func".to_string(),
                kind: super::super::SymbolKind::Function,
                kind_detail: "function".to_string(),
                file: "/test/mod.rs".to_string(),
                line: 1,
                column: 0,
                end_line: 1,
                end_column: 10,
                signature: None,
                doc_comment: None,
                visibility: None,
            },
        ];

        let path = PathBuf::from("/test/mod.rs");
        index.index_file(path.clone(), symbols);

        let locations = index.find_symbol("test_func");
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].file, path);
    }
}