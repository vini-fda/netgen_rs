// TODO: improve this main, using functions from lib.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(|s| s.as_str()), Some("-h" | "--help")) {
        println!(
            "Usage: netgen_rs TODO:params\n\
            - etc\n\
            - todo..."
        );
        return Ok(());
    }
    Ok(())
}
