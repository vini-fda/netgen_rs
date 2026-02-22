use std::io::{self, BufWriter, Read};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(|s| s.as_str()), Some("-h" | "--help")) {
        eprintln!(
            "Usage: netgen_rs [seed problem nodes sources sinks density mincost maxcost \
             supply tsources tsinks hicost% capacitated% mincap maxcap]\n\
             \n\
             Pass 15 arguments directly, or provide them via stdin (one or more problems,\n\
             whitespace-separated). Processing stops at EOF or when seed/problem <= 0."
        );
        return;
    }

    let input: String;
    let tokens: Box<dyn Iterator<Item = &str>>;

    if args.is_empty() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap();
        input = buf;
        tokens = Box::new(input.split_whitespace());
    } else {
        input = args.join(" ");
        tokens = Box::new(input.split_whitespace());
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut tokens = tokens.peekable();

    while let Some(seed) = tokens.next().and_then(|s| s.parse::<i64>().ok()) {
        if seed <= 0 {
            break;
        }

        let problem: i64 = match tokens.next().and_then(|s| s.parse().ok()) {
            Some(v) if v > 0 => v,
            _ => break,
        };

        let mut parms = [0i64; 13];
        for p in &mut parms {
            *p = match tokens.next().and_then(|s| s.parse().ok()) {
                Some(v) => v,
                None => {
                    eprintln!("Error: insufficient parameters");
                    std::process::exit(1);
                }
            };
        }

        let params = match netgen_rs::NetgenParams::from_slice(&parms) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };

        let result = match netgen_rs::generate(seed, &params) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };
        netgen_rs::write_dimacs(&mut out, seed, problem, &params, &result)
            .expect("writing DIMACS output");
    }
}
