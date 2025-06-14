use crate::indexer::Symbol;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn resolve_types_parallel(symbols: &HashMap<String, Symbol>) {
    symbols.par_iter().for_each(|(_, s)| {
        // Dummy resolution logic
        let _ = format!("Resolved symbol: {}", s.name);
    });
}
