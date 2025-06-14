use notify::{RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;

pub fn watch<F: FnMut(notify::Result<notify::Event>) + Send + 'static>(
    path: &Path,
    mut cb: F,
) -> Result<RecommendedWatcher> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        tx.send(res).unwrap();
    })?;
    watcher.watch(path, RecursiveMode::Recursive)?;
    std::thread::spawn(move || {
        for res in rx {
            cb(res);
        }
    });
    Ok(watcher)
}
