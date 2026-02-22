use std::io::{self, BufWriter, Read};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(|s| s.as_str()), Some("-h" | "--help")) {
        eprintln!(
            "Usage: netgen_rs < input_file\n\
             \n\
             Reads problems from stdin. Each problem is specified by 15 whitespace-separated\n\
             integers: seed, problem_number, followed by 13 parameters (nodes, sources, sinks,\n\
             density, mincost, maxcost, supply, tsources, tsinks, hicost%, capacitated%,\n\
             mincap, maxcap). Processing stops when seed <= 0 or problem <= 0."
        );
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_whitespace();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

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

        let params = netgen_rs::NetgenParams::from_slice(&parms);

        let result = match netgen_rs::generate(seed, &params) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };

        netgen_rs::write_dimacs(&mut out, seed, problem, &params, &result).unwrap();
    }
}
