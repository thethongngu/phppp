use crate::indexer::GlobalIndex;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn register(&self, _index: &GlobalIndex) {}
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }

    pub fn register_all(&self, index: &GlobalIndex) {
        for p in &self.plugins {
            tracing::info!("Registering plugin {}", p.name());
            p.register(index);
        }
    }
}
