use crate::indexer::Symbol;
use rayon::prelude::*;

pub fn resolve_types_parallel(symbols: &[Symbol]) {
    symbols.par_iter().for_each(|s| {
        // Dummy resolution logic
        let _ = format!("Resolved symbol: {}", s.name);
    });
}
