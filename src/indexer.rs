use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
}

pub fn extract_symbols(ast: &crate::parser::Ast) -> Vec<Symbol> {
    let mut result = Vec::new();
    collect_names(ast.0.root_node(), &mut result);
    result
}

fn collect_names(node: Node, out: &mut Vec<Symbol>) {
    if node.kind() == "name" {
        out.push(Symbol {
            name: "identifier".to_string(),
        });
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_names(child, out);
        }
    }
}
