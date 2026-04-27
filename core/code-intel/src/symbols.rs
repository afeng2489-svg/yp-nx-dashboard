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
                let mut symbol = Symbol {
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

                // Extract real signature for functions/methods
                if matches!(kind, SymbolKind::Function | SymbolKind::Method) {
                    symbol.signature = get_function_signature(&symbol, &parse_result.source);
                }

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
        "class_declaration" | "class_item" | "class" | "class_definition" => {
            Some(SymbolKind::Class)
        }
        "struct_declaration" | "struct_item" | "struct" | "struct_definition" => {
            Some(SymbolKind::Struct)
        }
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
    // Extract text from start of the symbol up to the opening brace
    let source_str = std::str::from_utf8(source).ok()?;
    let lines: Vec<&str> = source_str.lines().collect();

    // Symbol lines are 1-based, convert to 0-based
    let start_line = symbol.line.checked_sub(1)?;
    let end_line = symbol.end_line.checked_sub(1).unwrap_or(start_line);

    // Collect lines from start to end
    let mut sig_text = String::new();
    for line_idx in start_line..=end_line.min(lines.len().saturating_sub(1)) {
        let line = lines[line_idx];
        // Stop at opening brace — the signature is everything before it
        if let Some(brace_pos) = line.find('{') {
            let before_brace = line[..brace_pos].trim_end();
            if !before_brace.is_empty() {
                if !sig_text.is_empty() {
                    sig_text.push(' ');
                }
                sig_text.push_str(before_brace);
            }
            break;
        }
        if !sig_text.is_empty() {
            sig_text.push(' ');
        }
        sig_text.push_str(line.trim());
    }

    if sig_text.is_empty() {
        // Fallback: return just the symbol name
        Some(format!("fn {}", symbol.name))
    } else {
        Some(sig_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_mapping() {
        assert_eq!(
            map_node_kind_to_symbol("function_declaration"),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            map_node_kind_to_symbol("class_declaration"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            map_node_kind_to_symbol("struct_declaration"),
            Some(SymbolKind::Struct)
        );
        assert_eq!(map_node_kind_to_symbol("unknown"), None);
    }
}
