use std::{env, io, path::PathBuf};

use anyhow::Result;
use agent_wake::{
    detector::{Detector, DetectorConfig, KEYWORDS_FILE, MODEL_DIR},
    worker,
};

fn main() -> Result<()> {
    let model_dir = env::var("AGENT_WAKE_MODEL_DIR").unwrap_or_else(|_| MODEL_DIR.to_owned());
    let keywords_file = env::var("AGENT_WAKE_KEYWORDS_FILE").unwrap_or_else(|_| KEYWORDS_FILE.to_owned());
    let threshold = env::var("AGENT_WAKE_THRESHOLD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.25);
    let startup = Detector::load(DetectorConfig {
        model_dir: PathBuf::from(model_dir),
        keywords_file: PathBuf::from(keywords_file),
        threads: 1,
        threshold,
    })
    .map_err(|error| error.to_string());
    worker::run(io::stdin().lock(), io::stdout().lock(), startup)
}
