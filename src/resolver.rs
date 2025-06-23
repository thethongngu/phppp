use crate::indexer::{FileSymbols, GlobalIndex, Symbol, SymbolKind};
use crate::parser::Ast;
use regex::Regex;
use std::collections::HashMap;
use tower_lsp::lsp_types::{Location, Position, Url};
use tree_sitter::{Node, Point};

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
}

/// Normalize a symbol name into a fully qualified name using the current
/// namespace and `use` aliases.
fn normalize_name(name: &str, namespace: &str, aliases: &HashMap<String, String>) -> String {
    if name.starts_with("\\") {
        return name.trim_start_matches('\\').to_string();
    }

    let parts: Vec<&str> = name.split("\\").collect();
    if let Some(first) = parts.first() {
        if let Some(full) = aliases.get(*first) {
            let mut out = full.clone();
            if parts.len() > 1 {
                out.push_str("\\");
                out.push_str(&parts[1..].join("\\"));
            }
            return out;
        }
    }

    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}\\{}", namespace, name)
    }
}

/// Extract the namespace of the file, if any.
fn extract_namespace(src: &str) -> String {
    let re = Regex::new(r"(?m)^\s*namespace\s+([^;]+);?").unwrap();
    if let Some(cap) = re.captures(src) {
        cap[1].trim().to_string()
    } else {
        String::new()
    }
}

/// Extract `use` aliases from the source code.
fn extract_use_aliases(src: &str) -> HashMap<String, String> {
    let re = Regex::new(r"(?m)^\s*use\s+([^;]+);?").unwrap();
    let mut map = HashMap::new();
    for cap in re.captures_iter(src) {
        let clause = cap[1].trim();
        // parse individual use clause
        let mut parts = clause.split_whitespace();
        let path = parts.next().unwrap_or("");
        let alias = if let Some("as") = parts.next() {
            parts.next().unwrap_or("")
        } else {
            path.rsplit('\x5c').next().unwrap_or(path)
        };
        map.insert(alias.to_string(), path.to_string());
    }
    map
}

/// Find a local variable or parameter definition for `name` inside `ast` at
/// position `pos`.
fn find_local_variable(
    name: &str,
    src: &str,
    ast: &Ast,
    pos: Position,
    uri: &Url,
) -> Option<Symbol> {
    let root = ast.0.root_node();
    let point = Point {
        row: pos.line as usize,
        column: pos.character as usize,
    };
    let usage = root.descendant_for_point_range(point, point)?;

    // ascend to the nearest function-like node
    let mut current = usage;
    while current.kind() != "function_definition" && current.kind() != "method_declaration" {
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    if current.kind() == "function_definition" || current.kind() == "method_declaration" {
        // parameters
        if let Some(params) = current.child_by_field_name("parameters") {
            for i in 0..params.named_child_count() {
                if let Some(param) = params.named_child(i) {
                    if let Some(var) = param.child_by_field_name("name") {
                        if let Ok(text) = var.utf8_text(src.as_bytes()) {
                            if text.trim_start_matches('$') == name.trim_start_matches('$') {
                                return Some(Symbol {
                                    name: text.to_string(),
                                    kind: SymbolKind::Variable,
                                    location: node_location(uri, var),
                                    container: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn node_location(uri: &Url, node: Node) -> Location {
    let start = node.start_position();
    let end = node.end_position();
    Location {
        uri: uri.clone(),
        range: tower_lsp::lsp_types::Range {
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

/// Resolve a symbol using local scope, file symbols and the global index.
pub fn resolve_symbol(
    name: &str,
    uri: &Url,
    position: Position,
    src: &str,
    ast: &Ast,
    file_symbols: &FileSymbols,
    global: &GlobalIndex,
) -> Option<ResolvedSymbol> {
    tracing::debug!("Resolving symbol '{}' at {}:{}", name, uri, position.line);
    // Step 1: local variables/parameters
    if name.starts_with('$') {
        if let Some(sym) = find_local_variable(name, src, ast, position, uri) {
            return Some(ResolvedSymbol {
                name: sym.name,
                kind: sym.kind,
                location: sym.location,
            });
        }
    }

    let namespace = extract_namespace(src);
    let aliases = extract_use_aliases(src);
    let fqn = normalize_name(name, &namespace, &aliases);
    // resolved fully qualified name from aliases and namespace

    // Step 2: current file symbols
    if let Some(sym) = file_symbols.get(&fqn) {
        let resolved = ResolvedSymbol {
            name: sym.name.clone(),
            kind: sym.kind.clone(),
            location: sym.location.clone(),
        };
        tracing::debug!("Resolved symbol '{}' in current file", resolved.name);
        return Some(resolved);
    }

    // Step 3: global index
    for entry in global.iter() {
        if let Some(sym) = entry.value().get(&fqn) {
            let resolved = ResolvedSymbol {
                name: sym.name.clone(),
                kind: sym.kind.clone(),
                location: sym.location.clone(),
            };
            tracing::debug!("Resolved symbol '{}' in global index", resolved.name);
            return Some(resolved);
        }
    }

    tracing::debug!("Unable to resolve symbol '{}'", name);
    None
}
