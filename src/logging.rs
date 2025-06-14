use once_cell::sync::Lazy;
use std::sync::Mutex;
use tower_lsp::{Client, lsp_types::MessageType};

static CLIENT: Lazy<Mutex<Option<Client>>> = Lazy::new(|| Mutex::new(None));

struct ClientLogger;

impl log::Log for ClientLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let msg = format!("[{}] {}", record.level(), record.args());
        if let Some(client) = CLIENT.lock().unwrap().clone() {
            let _ = tokio::spawn(async move {
                let _ = client.log_message(MessageType::LOG, msg).await;
            });
        } else {
            eprintln!("{}", msg);
        }
    }

    fn flush(&self) {}
}

static LOGGER: ClientLogger = ClientLogger;

pub fn init(client: Client) {
    {
        let mut guard = CLIENT.lock().unwrap();
        *guard = Some(client);
    }
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
}
