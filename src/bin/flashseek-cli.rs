use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use flashseek::{Engine, HitJson};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut name_root: Option<PathBuf> = None;
    let mut content_root: Option<PathBuf> = None;
    let mut query = String::new();
    let mut json = false;
    let mut now = SystemTime::now();
    let mut out_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--name-root" => {
                i += 1;
                name_root = args.get(i).map(PathBuf::from);
            }
            "--content-root" => {
                i += 1;
                content_root = args.get(i).map(PathBuf::from);
            }
            "--query" => {
                i += 1;
                query = args.get(i).cloned().unwrap_or_default();
            }
            "--out" => {
                i += 1;
                out_path = args.get(i).map(PathBuf::from);
            }
            "--json" => json = true,
            "--now-epoch-secs" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    if let Ok(n) = s.parse::<u64>() {
                        now = SystemTime::UNIX_EPOCH + Duration::from_secs(n);
                    }
                }
            }
            "-h" | "--help" => {
                eprintln!(
                    "flashseek-cli --name-root DIR --content-root DIR --query TEXT [--json] [--now-epoch-secs N]"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(name_root) = name_root else {
        eprintln!("--name-root is required");
        return ExitCode::from(2);
    };
    let content_roots = match content_root {
        Some(p) => vec![p],
        None => Vec::new(),
    };
    let engine = match Engine::from_roots(&name_root, &content_roots) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("index failed: {e}");
            return ExitCode::from(1);
        }
    };
    let hits = engine.query(&query, now);
    let body = if json {
        let rows: Vec<HitJson> = hits.iter().map(HitJson::from).collect();
        match serde_json::to_string_pretty(&rows) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("json failed: {e}");
                return ExitCode::from(1);
            }
        }
    } else {
        hits.iter()
            .map(|h| h.record.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };
    if let Some(path) = out_path {
        if let Err(e) = std::fs::write(&path, body.as_bytes()) {
            eprintln!("write {}: {e}", path.display());
            return ExitCode::from(1);
        }
    } else {
        println!("{body}");
    }
    ExitCode::SUCCESS
}
