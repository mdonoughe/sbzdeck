use slog::Logger;
use sloggers::file::FileLoggerBuilder;
use sloggers::types::Severity;
use sloggers::Build;
use std::env;

pub fn create() -> Logger {
    let mut log_path = env::current_exe()
        .ok()
        .and_then(|mut p| if p.pop() { Some(p) } else { None })
        .unwrap_or_else(env::temp_dir);
    log_path.push("sbzdeck.log");
    let mut logger = FileLoggerBuilder::new(log_path);
    logger.level(Severity::Debug);
    logger.build().unwrap()
}
