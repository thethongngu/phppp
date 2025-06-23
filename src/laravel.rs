use crate::indexer::GlobalIndex;
use crate::plugin::Plugin;

pub struct LaravelPlugin;

impl Plugin for LaravelPlugin {
    fn name(&self) -> &str {
        "laravel"
    }

    fn register(&self, _index: &GlobalIndex) {
        log::info!("Laravel helper plugin active");
    }
}
