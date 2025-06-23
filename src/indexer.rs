use std::collections::HashMap;
use std::fs;
use std::path::Path;

use dashmap::DashMap;
use tower_lsp::lsp_types::{Location, Position, Range, Url};
use tree_sitter::Node;

use crate::parser::{self, Ast};
use bumpalo::Bump;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Class,
    Constant,
    Variable,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<String>,
}

pub type FileSymbols = HashMap<String, Symbol>;
pub type GlobalIndex = std::sync::Arc<DashMap<Url, FileSymbols>>;

pub fn new_index() -> GlobalIndex {
    std::sync::Arc::new(DashMap::new())
}

pub fn scan_workspace(root: &Path, index: &GlobalIndex) -> std::io::Result<()> {
    index.clear();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("php") {
            index_file(entry.path(), index)?;
        }
    }
    Ok(())
}

pub fn index_file(path: &Path, index: &GlobalIndex) -> std::io::Result<()> {
    if !path.exists() {
        index.remove(&Url::from_file_path(path).unwrap());
        return Ok(());
    }
    if path.extension().and_then(|s| s.to_str()) != Some("php") {
        return Ok(());
    }
    let src = fs::read_to_string(path)?;
    let bump = Bump::new();
    let ast = parser::parse_php(&src, &bump);
    let uri = Url::from_file_path(path).unwrap();
    let symbols = extract_symbols(&src, &ast, &uri);
    index.insert(uri, symbols);
    Ok(())
}

pub fn extract_symbols(src: &str, ast: &Ast, uri: &Url) -> FileSymbols {
    tracing::debug!("Indexing symbols in {}", uri);
    let root = ast.0.root_node();
    let mut out = HashMap::new();
    let mut namespace = String::new();
    for i in 0..root.named_child_count() {
        if let Some(child) = root.named_child(i) {
            match child.kind() {
                "namespace_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(ns) = name_node.utf8_text(src.as_bytes()) {
                            namespace = ns.to_string();
                        }
                    }
                }
                _ => collect_node(src, child, uri, &namespace, &mut out),
            }
        }
    }
    tracing::debug!("Found {} symbols", out.len());
    out
}

fn collect_node(src: &str, node: Node, uri: &Url, namespace: &str, out: &mut FileSymbols) {
    match node.kind() {
        "function_definition" => {
            add_symbol(src, node, uri, namespace, SymbolKind::Function, out);
        }
        "class_declaration" => {
            add_symbol(src, node, uri, namespace, SymbolKind::Class, out);
        }
        "const_declaration" => add_constant(src, node, uri, namespace, out),
        "expression_statement" => {
            if let Some(expr) = node.named_child(0) {
                if expr.kind() == "assignment_expression" {
                    add_variable(src, expr, uri, namespace, out);
                }
            }
        }
        _ => {}
    }
}

fn add_symbol(
    src: &str,
    node: Node,
    uri: &Url,
    namespace: &str,
    kind: SymbolKind,
    out: &mut FileSymbols,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(src.as_bytes()) {
            let fqn = if namespace.is_empty() {
                name.to_string()
            } else {
                format!("{}\\{}", namespace, name)
            };
            out.insert(
                fqn.clone(),
                Symbol {
                    name: fqn,
                    kind,
                    location: node_location(uri, node),
                    container: None,
                },
            );
        }
    }
}

fn add_constant(src: &str, node: Node, uri: &Url, namespace: &str, out: &mut FileSymbols) {
    for i in 0..node.named_child_count() {
        if let Some(constant) = node.named_child(i) {
            if constant.kind() == "const_element" {
                let name_node = constant
                    .child_by_field_name("name")
                    .or_else(|| constant.named_child(0));
                if let Some(name_node) = name_node {
                    if let Ok(name) = name_node.utf8_text(src.as_bytes()) {
                        let fqn = if namespace.is_empty() {
                            name.to_string()
                        } else {
                            format!("{}\\{}", namespace, name)
                        };
                        out.insert(
                            fqn.clone(),
                            Symbol {
                                name: fqn,
                                kind: SymbolKind::Constant,
                                location: node_location(uri, name_node),
                                container: None,
                            },
                        );
                    }
                }
            }
        }
    }
}

fn add_variable(src: &str, node: Node, uri: &Url, namespace: &str, out: &mut FileSymbols) {
    if let Some(left) = node.child_by_field_name("left") {
        if left.kind() == "variable_name" {
            if let Ok(name) = left.utf8_text(src.as_bytes()) {
                let fqn = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{}\\{}", namespace, name)
                };
                out.insert(
                    fqn.clone(),
                    Symbol {
                        name: fqn,
                        kind: SymbolKind::Variable,
                        location: node_location(uri, left),
                        container: None,
                    },
                );
            }
        }
    }
}

fn node_location(uri: &Url, node: Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        uri: uri.clone(),
        range: Range {
            start: Position {
                line: start.row as u32,
                character: start.column as u32,
            },
            end: Position {
                line: end.row as u32,
                character: end.column as u32,
            },
        },
    }
}
