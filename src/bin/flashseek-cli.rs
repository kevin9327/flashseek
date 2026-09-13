use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use flashseek::{Engine, Hit, HitJson};

const HELP: &str = "\
flashseek-cli --name-root DIR --content-root DIR --query TEXT [--json] [--now-epoch-secs N] [--out PATH] [--max-results N] [--files-only] [--folders-only] [--config FILE] [--count] [--quiet]
  --max-results N  max hits after search (0 = unlimited)
  --files-only     keep files only
  --folders-only   keep folders only
  --config FILE    name_root=/ content_root=/ lines; flags override
  --count          print hit total after filters (wins over --json)
  --quiet          with --count, suppress non-JSON path lists
";

#[derive(Debug)]
struct Cli {
    name_root: Option<PathBuf>,
    content_root: Option<PathBuf>,
    query: String,
    json: bool,
    now: SystemTime,
    out_path: Option<PathBuf>,
    /// 0 = unlimited
    max_results: usize,
    files_only: bool,
    folders_only: bool,
    help: bool,
    config_path: Option<PathBuf>,
    count: bool,
    quiet: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            name_root: None,
            content_root: None,
            query: String::new(),
            json: false,
            now: SystemTime::now(),
            out_path: None,
            max_results: 0,
            files_only: false,
            folders_only: false,
            help: false,
            config_path: None,
            count: false,
            quiet: false,
        }
    }
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--name-root" => {
                i += 1;
                cli.name_root = args.get(i).map(PathBuf::from);
            }
            "--content-root" => {
                i += 1;
                cli.content_root = args.get(i).map(PathBuf::from);
            }
            "--query" => {
                i += 1;
                cli.query = args.get(i).cloned().unwrap_or_default();
            }
            "--out" => {
                i += 1;
                cli.out_path = args.get(i).map(PathBuf::from);
            }
            "--json" => cli.json = true,
            "--now-epoch-secs" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    if let Ok(n) = s.parse::<u64>() {
                        cli.now = SystemTime::UNIX_EPOCH + Duration::from_secs(n);
                    }
                }
            }
            "--max-results" => {
                i += 1;
                let Some(s) = args.get(i) else {
                    return Err("--max-results requires a number".into());
                };
                cli.max_results = s
                    .parse()
                    .map_err(|_| format!("invalid --max-results: {s}"))?;
            }
            "--files-only" => cli.files_only = true,
            "--folders-only" => cli.folders_only = true,
            "--count" => cli.count = true,
            "--quiet" => cli.quiet = true,
            "--config" => {
                i += 1;
                cli.config_path = args.get(i).map(PathBuf::from);
            }
            "-h" | "--help" => {
                cli.help = true;
                return Ok(cli);
            }
            other => return Err(format!("unknown arg: {other}")),
        }
        i += 1;
    }
    if cli.files_only && cli.folders_only {
        return Err("cannot combine --files-only and --folders-only".into());
    }
    Ok(cli)
}

fn apply_cli_filters(mut hits: Vec<Hit>, cli: &Cli) -> Vec<Hit> {
    if cli.files_only {
        hits.retain(|h| !h.record.is_dir);
    } else if cli.folders_only {
        hits.retain(|h| h.record.is_dir);
    }
    if cli.max_results > 0 {
        hits.truncate(cli.max_results);
    }
    hits
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_args(&args) {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if cli.help {
        eprintln!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let mut name_root = cli.name_root.clone();
    let mut content_roots = match cli.content_root.clone() {
        Some(p) => vec![p],
        None => Vec::new(),
    };
    if let Some(cfg_path) = &cli.config_path {
        let text = match std::fs::read_to_string(cfg_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("config {}: {e}", cfg_path.display());
                return ExitCode::from(2);
            }
        };
        match flashseek::parse_engine_config(&text) {
            Ok(cfg) => {
                if name_root.is_none() {
                    name_root = Some(cfg.name_root);
                }
                if content_roots.is_empty() {
                    content_roots = cfg.content_roots;
                }
            }
            Err(e) => {
                eprintln!("config parse: {e}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(name_root) = name_root else {
        eprintln!("--name-root is required (or set it in --config)");
        return ExitCode::from(2);
    };
    let engine = match Engine::from_roots(&name_root, &content_roots) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("index failed: {e}");
            return ExitCode::from(1);
        }
    };
    let hits = apply_cli_filters(engine.query(&cli.query, cli.now), &cli);
    // --count wins over --json: print hits.len() as decimal, skip paths/json.
    let body = if cli.count {
        hits.len().to_string()
    } else if cli.json {
        let rows: Vec<HitJson> = hits.iter().map(HitJson::from).collect();
        match serde_json::to_string_pretty(&rows) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("json failed: {e}");
                return ExitCode::from(1);
            }
        }
    } else if cli.quiet {
        String::new()
    } else {
        hits.iter()
            .map(|h| h.record.path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };
    if let Some(path) = cli.out_path {
        if let Err(e) = std::fs::write(&path, body.as_bytes()) {
            eprintln!("write {}: {e}", path.display());
            return ExitCode::from(1);
        }
    } else if cli.count || cli.json || !cli.quiet {
        println!("{body}");
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashseek::FileRecord;

    fn args(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    fn hit(name: &str, is_dir: bool) -> Hit {
        Hit {
            record: FileRecord {
                id: 1,
                parent_id: None,
                name: name.into(),
                path: PathBuf::from(name),
                size: 0,
                modified: SystemTime::UNIX_EPOCH,
                is_dir,
                attributes: 0,
            },
            score: 0.0,
            name_match: true,
            path_match: false,
            content_match: false,
            snippet: None,
        }
    }

    fn sample_hits() -> Vec<Hit> {
        vec![
            hit("dir-a", true),
            hit("file-a.txt", false),
            hit("file-b.txt", false),
            hit("dir-b", true),
            hit("file-c.txt", false),
        ]
    }

    #[test]
    fn defaults_are_unlimited_and_unfiltered() {
        let cli = parse_args(&args(&["--query", "notes"])).unwrap();
        assert_eq!(cli.query, "notes");
        assert_eq!(cli.max_results, 0);
        assert!(!cli.files_only);
        assert!(!cli.folders_only);
        assert!(!cli.help);
        assert!(!cli.json);
        assert!(!cli.count);
        assert!(!cli.quiet);
    }

    #[test]
    fn max_results_parses() {
        let cli = parse_args(&args(&["--max-results", "10"])).unwrap();
        assert_eq!(cli.max_results, 10);
    }

    #[test]
    fn max_results_zero_is_unlimited() {
        let cli = parse_args(&args(&["--max-results", "0"])).unwrap();
        assert_eq!(cli.max_results, 0);
        let hits = apply_cli_filters(sample_hits(), &cli);
        assert_eq!(hits.len(), 5);
    }

    #[test]
    fn max_results_missing_value_errors() {
        let err = parse_args(&args(&["--max-results"])).unwrap_err();
        assert!(err.contains("--max-results"));
    }

    #[test]
    fn max_results_invalid_value_errors() {
        let err = parse_args(&args(&["--max-results", "nope"])).unwrap_err();
        assert!(err.contains("invalid --max-results"));
    }

    #[test]
    fn files_only_flag() {
        let cli = parse_args(&args(&["--files-only"])).unwrap();
        assert!(cli.files_only);
        assert!(!cli.folders_only);
    }

    #[test]
    fn folders_only_flag() {
        let cli = parse_args(&args(&["--folders-only"])).unwrap();
        assert!(cli.folders_only);
        assert!(!cli.files_only);
    }

    #[test]
    fn files_and_folders_only_conflict() {
        let err = parse_args(&args(&["--files-only", "--folders-only"])).unwrap_err();
        assert!(err.contains("--files-only"));
        assert!(err.contains("--folders-only"));
    }

    #[test]
    fn unknown_arg_errors() {
        let err = parse_args(&args(&["--nope"])).unwrap_err();
        assert_eq!(err, "unknown arg: --nope");
    }

    #[test]
    fn help_flag_short_and_long() {
        let long = parse_args(&args(&["--help"])).unwrap();
        assert!(long.help);
        let short = parse_args(&args(&["-h"])).unwrap();
        assert!(short.help);
    }

    #[test]
    fn help_text_lists_new_flags() {
        assert!(HELP.contains("--max-results"));
        assert!(HELP.contains("--files-only"));
        assert!(HELP.contains("--folders-only"));
        assert!(HELP.contains("--config"));
        assert!(HELP.contains("--count"));
        assert!(HELP.contains("--quiet"));
    }

    #[test]
    fn count_flag() {
        let cli = parse_args(&args(&["--count"])).unwrap();
        assert!(cli.count);
        assert!(!cli.quiet);
        assert!(!cli.json);
    }

    #[test]
    fn quiet_flag() {
        let cli = parse_args(&args(&["--quiet"])).unwrap();
        assert!(cli.quiet);
        assert!(!cli.count);
    }

    #[test]
    fn count_and_quiet_together() {
        let cli = parse_args(&args(&["--count", "--quiet"])).unwrap();
        assert!(cli.count);
        assert!(cli.quiet);
    }

    #[test]
    fn count_and_json_both_parse_count_wins_at_output() {
        let cli = parse_args(&args(&["--json", "--count"])).unwrap();
        assert!(cli.json);
        assert!(cli.count);
    }

    #[test]
    fn config_flag_parses() {
        let cli = parse_args(&args(&["--config", "C:\\flashseek.conf", "--query", "pdf"])).unwrap();
        assert_eq!(cli.config_path.unwrap(), PathBuf::from("C:\\flashseek.conf"));
        assert_eq!(cli.query, "pdf");
    }

    #[test]
    fn existing_flags_still_parse() {
        let cli = parse_args(&args(&[
            "--name-root",
            "C:\\data",
            "--content-root",
            "C:\\docs",
            "--query",
            "세금 pdf",
            "--json",
            "--out",
            "hits.json",
            "--now-epoch-secs",
            "100",
            "--max-results",
            "3",
            "--files-only",
        ]))
        .unwrap();
        assert_eq!(cli.name_root.unwrap(), PathBuf::from("C:\\data"));
        assert_eq!(cli.content_root.unwrap(), PathBuf::from("C:\\docs"));
        assert_eq!(cli.query, "세금 pdf");
        assert!(cli.json);
        assert_eq!(cli.out_path.unwrap(), PathBuf::from("hits.json"));
        assert_eq!(cli.now, SystemTime::UNIX_EPOCH + Duration::from_secs(100));
        assert_eq!(cli.max_results, 3);
        assert!(cli.files_only);
    }

    #[test]
    fn files_only_keeps_files() {
        let cli = parse_args(&args(&["--files-only"])).unwrap();
        let hits = apply_cli_filters(sample_hits(), &cli);
        let names: Vec<_> = hits.iter().map(|h| h.record.name.as_str()).collect();
        assert_eq!(names, ["file-a.txt", "file-b.txt", "file-c.txt"]);
        assert!(hits.iter().all(|h| !h.record.is_dir));
    }

    #[test]
    fn folders_only_keeps_dirs() {
        let cli = parse_args(&args(&["--folders-only"])).unwrap();
        let hits = apply_cli_filters(sample_hits(), &cli);
        let names: Vec<_> = hits.iter().map(|h| h.record.name.as_str()).collect();
        assert_eq!(names, ["dir-a", "dir-b"]);
        assert!(hits.iter().all(|h| h.record.is_dir));
    }

    #[test]
    fn max_results_truncates_after_filter() {
        let cli = parse_args(&args(&["--files-only", "--max-results", "2"])).unwrap();
        let hits = apply_cli_filters(sample_hits(), &cli);
        let names: Vec<_> = hits.iter().map(|h| h.record.name.as_str()).collect();
        assert_eq!(names, ["file-a.txt", "file-b.txt"]);
    }
}
