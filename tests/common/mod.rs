use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

static BUILD_C: Once = Once::new();

fn c_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-c")
        .join("netgen_c")
}

fn build_c_reference() {
    BUILD_C.call_once(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_dir = manifest_dir.join("netgen_original");
        let out_dir = manifest_dir.join("target").join("test-c");
        std::fs::create_dir_all(&out_dir).unwrap();

        let tmp_dir = out_dir.join("patched");
        std::fs::create_dir_all(&tmp_dir).unwrap();

        for file in &["netgen.c", "index.c", "random.c", "netgen.h"] {
            let content = std::fs::read_to_string(c_dir.join(file)).unwrap();
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
            *p = match tokens.next() {
                Some(val) => val.parse().unwrap(),
                None => return result,
            };
        }

        let params = netgen_rs::NetgenParams::from_slice(&parms).unwrap();
        let gen_result = netgen_rs::generate(seed, &params).unwrap();
        let mut buf = Vec::new();
        netgen_rs::write_dimacs(&mut buf, seed, problem, &params, &gen_result).unwrap();
        result.push_str(&String::from_utf8(buf).unwrap());
    }

    result
}

pub fn assert_identical(input: &str) {
    let c_out = run_c(input);
    let rust_out = run_rust(input);
    assert_eq!(
        c_out,
        rust_out,
        "Output mismatch for input: {}",
        input.trim()
    );
}
