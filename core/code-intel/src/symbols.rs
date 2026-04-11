//! 从解析的代码中提取符号

use super::{ParseResult, TreeNode};
use serde::{Deserialize, Serialize};

/// 符号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// 函数
    Function,
    /// 方法
    Method,
    /// 类
    Class,
    /// 结构体
    Struct,
    /// 枚举
    Enum,
    /// 特质
    Trait,
    /// 模块
    Module,
    /// 常量
    Constant,
    /// 变量
    Variable,
    /// 类型
    Type,
    /// 导入
    Import,
    /// 属性
    Property,
}

/// 从代码中提取的符号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// 符号名称
    pub name: String,
    /// 符号类型
    pub kind: SymbolKind,
    /// 类型详情
    pub kind_detail: String,
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
    /// 结束行号
    pub end_line: usize,
    /// 结束列号
    pub end_column: usize,
    /// 函数签名
    pub signature: Option<String>,
    /// 文档注释
    pub doc_comment: Option<String>,
    /// 可见性
    pub visibility: Option<String>,
}

/// 符号提取器
pub struct SymbolExtractor;

impl SymbolExtractor {
    /// 从解析结果中提取所有符号
    pub fn extract(parse_result: &ParseResult) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let nodes = super::walk_tree(&parse_result.tree, &parse_result.source);

        for node in nodes {
            if let Some(kind) = map_node_kind_to_symbol(&node.kind) {
                let symbol = Symbol {
                    name: node.text.clone(),
                    kind,
                    kind_detail: node.kind.clone(),
                    file: parse_result.path.clone(),
                    line: node.start_point.0 + 1,
                    column: node.start_point.1,
                    end_line: node.end_point.0 + 1,
                    end_column: node.end_point.1,
                    signature: None,
                    doc_comment: None,
                    visibility: None,
                };
                symbols.push(symbol);
            }
        }

        symbols
    }
}

/// 将节点类型映射为符号类型
fn map_node_kind_to_symbol(kind: &str) -> Option<SymbolKind> {
    match kind {
        // 常见函数模式
        "function_declaration" | "function_item" | "function" => Some(SymbolKind::Function),
        "method_declaration" | "method_definition" | "function_method" => Some(SymbolKind::Method),
        "class_declaration" | "class_item" | "class" | "class_definition" => Some(SymbolKind::Class),
        "struct_declaration" | "struct_item" | "struct" | "struct_definition" => Some(SymbolKind::Struct),
        "enum_declaration" | "enum_item" | "enum" => Some(SymbolKind::Enum),
        "trait_declaration" | "trait_item" | "trait" => Some(SymbolKind::Trait),
        "module_declaration" | "module" | "namespace" => Some(SymbolKind::Module),
        "const_declaration" | "const_item" | "constant" => Some(SymbolKind::Constant),
        "let_declaration" | "let_item" | "variable_declaration" => Some(SymbolKind::Variable),
        "type_declaration" | "type_item" | "type_alias" => Some(SymbolKind::Type),
        "import_declaration" | "import" | "use_declaration" => Some(SymbolKind::Import),
        "property_declaration" | "property" | "field_declaration" => Some(SymbolKind::Property),
        _ => None,
    }
}

/// 从函数符号获取函数签名
pub fn get_function_signature(symbol: &Symbol, source: &[u8]) -> Option<String> {
    // 在实际实现中，这会提取参数名和类型
    Some(format!("fn {}", symbol.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_mapping() {
        assert_eq!(map_node_kind_to_symbol("function_declaration"), Some(SymbolKind::Function));
        assert_eq!(map_node_kind_to_symbol("class_declaration"), Some(SymbolKind::Class));
        assert_eq!(map_node_kind_to_symbol("struct_declaration"), Some(SymbolKind::Struct));
        assert_eq!(map_node_kind_to_symbol("unknown"), None);
    }
}