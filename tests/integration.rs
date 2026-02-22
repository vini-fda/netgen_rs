use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

static BUILD_C: Once = Once::new();

fn c_binary_path() -> PathBuf {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-c");
    out_dir.join("netgen_c")
}

fn build_c_reference() {
    BUILD_C.call_once(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_dir = manifest_dir.join("netgen_original");
        let out_dir = manifest_dir.join("target").join("test-c");
        std::fs::create_dir_all(&out_dir).unwrap();

        // Create patched copies with random -> ng_random to avoid macOS conflict
        let tmp_dir = out_dir.join("patched");
        std::fs::create_dir_all(&tmp_dir).unwrap();

        for file in &["netgen.c", "index.c", "random.c", "netgen.h"] {
            let content = std::fs::read_to_string(c_dir.join(file)).unwrap();
            // Use a simple word-boundary replacement: random( -> ng_random(
            // and "long random" -> "long ng_random", but preserve "set_random"
            let patched = content
                .replace("random(", "ng_random(")
                .replace("set_ng_random", "set_random");
            std::fs::write(tmp_dir.join(file), patched).unwrap();
        }

        let status = Command::new("cc")
            .args([
                "-O",
                "-DDIMACS",
                "-w",
                &format!("-I{}", tmp_dir.display()),
                "-o",
                &c_binary_path().to_string_lossy(),
            ])
            .arg(tmp_dir.join("netgen.c"))
            .arg(tmp_dir.join("index.c"))
            .arg(tmp_dir.join("random.c"))
            .status()
            .expect("Failed to invoke C compiler");

        assert!(
            status.success(),
            "Failed to compile C reference implementation"
        );
    });
}

fn run_c(input: &str) -> String {
    build_c_reference();
    let mut child = Command::new(c_binary_path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run C binary");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn run_rust(input: &str) -> String {
    // Parse the input the same way main.rs does
    let mut tokens = input.split_whitespace();
    let mut result = String::new();

    loop {
        let seed: i64 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(v) if v > 0 => v,
            _ => break,
        };
        let problem: i64 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(v) if v > 0 => v,
            _ => break,
        };
        let mut parms = [0i64; 13];
        for p in &mut parms {
            *p = tokens.next().unwrap().parse().unwrap();
        }

        let params = netgen_rs::NetgenParams::from_slice(&parms);
        let gen_result = netgen_rs::generate(seed, &params).unwrap();
        let mut buf = Vec::new();
        netgen_rs::write_dimacs(&mut buf, seed, problem, &params, &gen_result).unwrap();
        result.push_str(&String::from_utf8(buf).unwrap());
    }

    result
}

fn assert_identical(input: &str) {
    let c_out = run_c(input);
    let rust_out = run_rust(input);
    assert_eq!(
        c_out,
        rust_out,
        "Output mismatch for input: {}",
        input.trim()
    );
}

#[test]
fn min_cost_flow_small() {
    assert_identical("13502460 1 512 2 2 1000 10 100 200 0 0 20 100 10 1000\n");
}

#[test]
fn assignment_problem() {
    assert_identical("12345 1 100 50 50 500 1 100 50 0 0 0 0 1 100\n");
}

#[test]
fn max_flow_problem() {
    assert_identical("99999 1 200 5 5 1000 1 1 500 2 2 20 50 10 100\n");
}

#[test]
fn min_cost_flow_large() {
    assert_identical("7654321 1 1024 10 10 5000 5 500 1000 3 3 30 80 50 2000\n");
}

#[test]
fn stress_8k_nodes() {
    assert_identical("13502460 1 8192 50 50 50000 1 1000 10000 10 10 25 75 100 5000\n");
}

#[test]
fn multi_problem_sequence() {
    assert_identical(
        "13502460 1 512 2 2 1000 10 100 200 0 0 20 100 10 1000\n\
         12345 2 100 50 50 500 1 100 50 0 0 0 0 1 100\n",
    );
}

#[test]
fn assignment_large() {
    assert_identical("42 1 200 100 100 2000 10 500 100 0 0 0 0 1 1\n");
}

#[test]
fn various_seeds() {
    for seed in [1, 42, 1000, 999999, 2147483646] {
        let input = format!("{seed} 1 256 4 4 2000 1 100 500 1 1 10 50 5 200\n");
        assert_identical(&input);
    }
}
