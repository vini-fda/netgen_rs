mod common;

use common::assert_identical;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_NODES: i64 = 65_536;
const MAX_CASES_PER_SCRIPT: usize = 8;

#[test]
fn lemon_netgen_8_suite() {
    run_lemon_suite("netgen_8.sh");
}

#[test]
fn lemon_netgen_sr_suite() {
    run_lemon_suite("netgen_sr.sh");
}

#[test]
fn lemon_netgen_lo_8_suite() {
    run_lemon_suite("netgen_lo_8.sh");
}

#[test]
fn lemon_netgen_lo_sr_suite() {
    run_lemon_suite("netgen_lo_sr.sh");
}

#[test]
fn lemon_netgen_deg_suite() {
    run_lemon_suite("netgen_deg.sh");
}

#[derive(Clone)]
struct Case {
    name: String,
    line: String,
    nodes: i64,
}

fn run_lemon_suite(script_name: &str) {
    let cases = load_cases(script_name);
    assert!(
        !cases.is_empty(),
        "No usable cases found in {}",
        script_name
    );
    for case in cases {
        let input = format!("{}\n", case.line);
        assert_identical(&input);
    }
}

fn load_cases(script_name: &str) -> Vec<Case> {
    let script_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("lemon_scripts")
        .join(script_name);
    let temp_dir = create_temp_dir(script_name);

    let status = Command::new("bash")
        .arg(&script_path)
        .current_dir(&temp_dir)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {}", script_name, e));
    assert!(status.success(), "script {} failed", script_name);

    let mut cases: Vec<Case> = read_param_files(&temp_dir);
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    cases.retain(|case| case.nodes <= MAX_NODES);
    if cases.len() > MAX_CASES_PER_SCRIPT {
        cases.truncate(MAX_CASES_PER_SCRIPT);
    }

    let _ = fs::remove_dir_all(&temp_dir);
    cases
}

fn read_param_files(dir: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    for entry in fs::read_dir(dir).expect("reading temp dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("param") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("param file");
        let line = content.trim();
        if line.is_empty() {
            continue;
        }
        let nodes = line
            .split_whitespace()
            .nth(2)
            .expect("nodes field")
            .parse()
            .expect("nodes number");
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        cases.push(Case {
            name,
            line: line.to_string(),
            nodes,
        });
    }
    cases
}

fn create_temp_dir(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.push(format!(
        "netgen_lemon_{}_{}_{}",
        label.replace('/', "_"),
        process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
