//! 代码中的引用查找

use super::{ParseResult, Symbol, TreeNode};

/// 对符号的引用
#[derive(Debug, Clone)]
pub struct Reference {
    /// 符号名称
    pub symbol_name: String,
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
    /// 引用类型
    pub reference_type: ReferenceType,
    /// 引用该符号的文本
    pub text: String,
}

/// 引用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    /// 直接读取符号
    Read,
    /// 写入符号
    Write,
    /// 调用符号（用于函数/方法）
    Call,
    /// 类型引用（用于类型/类）
    TypeReference,
    /// 导入语句
    Import,
}

/// 引用查找器
pub struct ReferenceFinder;

impl ReferenceFinder {
    /// 在解析结果中查找符号的所有引用
    pub fn find_references(parse_result: &ParseResult, symbol: &Symbol) -> Vec<Reference> {
        let mut references = Vec::new();
        let nodes = super::walk_tree(&parse_result.tree, &parse_result.source);

        for node in nodes {
            if node.text == symbol.name && node.kind != symbol.kind_detail {
                let ref_type = classify_reference(&node, symbol);
                references.push(Reference {
                    symbol_name: symbol.name.clone(),
                    file: parse_result.path.clone(),
                    line: node.start_point.0 + 1,
                    column: node.start_point.1,
                    reference_type: ref_type,
                    text: node.text.clone(),
                });
            }
        }

        references
    }

    /// 在多个文件中查找符号的所有引用
    pub fn find_in_project(files: &[ParseResult], symbol_name: &str) -> Vec<Reference> {
        let mut all_references = Vec::new();

        for file in files {
            let nodes = super::walk_tree(&file.tree, &file.source);
            for node in nodes {
                if node.text == symbol_name {
                    all_references.push(Reference {
                        symbol_name: symbol_name.to_string(),
                        file: file.path.clone(),
                        line: node.start_point.0 + 1,
                        column: node.start_point.1,
                        reference_type: ReferenceType::Read,
                        text: node.text.clone(),
                    });
                }
            }
        }

        all_references
    }
}

/// 对引用进行分类
fn classify_reference(node: &TreeNode, symbol: &Symbol) -> ReferenceType {
    let parent_kind = node.kind.as_str();
    match symbol.kind {
        super::SymbolKind::Function | super::SymbolKind::Method => {
            if parent_kind.contains("call") {
                ReferenceType::Call
            } else {
                ReferenceType::Read
            }
        }
        super::SymbolKind::Variable | super::SymbolKind::Constant => {
            if parent_kind.contains("assignment") || parent_kind.contains("declaration") {
                ReferenceType::Write
            } else {
                ReferenceType::Read
            }
        }
        super::SymbolKind::Type | super::SymbolKind::Class | super::SymbolKind::Struct => {
            if parent_kind.contains("type") || parent_kind.contains("annotation") {
                ReferenceType::TypeReference
            } else {
                ReferenceType::Read
            }
        }
        super::SymbolKind::Import => ReferenceType::Import,
        _ => ReferenceType::Read,
    }
}