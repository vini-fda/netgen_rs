# netgen-rs

A Rust port of [NETGEN](http://archive.dimacs.rutgers.edu/pub/netflow/generators/network/netgen/), the classic network flow problem generator described in:

> Klingman, Napier, and Stutz, "NETGEN: A program for generating large scale capacitated assignment, transportation, and minimum-cost flow network problems," *Management Science* 20, 814–820 (1974).

Generates minimum-cost flow, assignment, and maximum flow problems in [DIMACS format](http://lpsolve.sourceforge.net/5.5/DIMACS_mcf.htm). Produces **byte-identical output** to the reference C implementation for the same inputs.

## Install

```sh
cargo install --path .
```

## CLI usage

Parameters can be passed as **command-line arguments** or via **stdin**. Each problem requires 15 whitespace-separated integers:

```
seed  problem_number  nodes  sources  sinks  density  mincost  maxcost  supply  tsources  tsinks  hicost%  capacitated%  mincap  maxcap
```

```sh
# Pass parameters directly as arguments
netgen_rs 13502460 1 512 10 10 2000 5 500 1000 3 3 20 80 50 2000

# Or pipe via stdin (supports multiple problems in sequence)
echo "13502460 1 512 10 10 2000 5 500 1000 3 3 20 80 50 2000" | netgen_rs

# Assignment problem (sources+sinks=nodes, sources=sinks, supply=sources)
netgen_rs 12345 1 100 50 50 500 1 100 50 0 0 0 0 1 100

# Max flow problem (mincost=maxcost=1)
netgen_rs 99999 1 200 5 5 1000 1 1 500 2 2 20 50 10 100

# Multiple problems from a file
netgen_rs < problems.txt > output.dimacs
```

When no arguments are given, `netgen_rs` reads from stdin. Processing stops at EOF or when seed/problem ≤ 0.

### Parameters

| Parameter | Description |
|-----------|-------------|
| `seed` | Positive random seed (deterministic output for same seed) |
| `problem_number` | Problem identifier (appears in output header) |
| `nodes` | Total number of nodes |
| `sources` | Number of source nodes (including transshipment) |
| `sinks` | Number of sink nodes (including transshipment) |
| `density` | Number of arcs to generate |
| `mincost` | Minimum arc cost |
| `maxcost` | Maximum arc cost |
| `supply` | Total supply across all sources |
| `tsources` | Number of transshipment sources |
| `tsinks` | Number of transshipment sinks |
| `hicost%` | Percentage of skeleton arcs assigned maximum cost (0–100) |
| `capacitated%` | Percentage of arcs to be capacitated (0–100) |
| `mincap` | Minimum arc capacity |
| `maxcap` | Maximum arc capacity |

### Problem type detection

The problem type is inferred from the parameters (matching the original NETGEN behavior):

- **Assignment** — `sources + sinks = nodes`, `sources = sinks`, `supply = sources`, and no transshipment nodes
- **Maximum flow** — `mincost = maxcost = 1`
- **Minimum-cost flow** — everything else

## Library usage

Add the library to your `Cargo.toml`:

```sh
cargo add netgen_rs
```

### Generate a network

```rust
use netgen_rs::{NetgenParams, generate};

let params = NetgenParams {
    nodes: 512,
    sources: 10,
    sinks: 10,
    density: 2000,
    mincost: 5,
    maxcost: 500,
    supply: 1000,
    tsources: 3,
    tsinks: 3,
    hicost_pct: 20,
    capacitated_pct: 80,
    mincap: 50,
    maxcap: 2000,
};

let result = generate(13502460, &params).unwrap();

for arc in &result.arcs {
    println!("{} -> {}: cost={}, cap={}", arc.from, arc.to, arc.cost, arc.capacity);
}

for (i, &s) in result.supply.iter().enumerate() {
    if s != 0 {
        println!("node {}: supply={}", i + 1, s);
    }
}
```

### Write DIMACS output

```rust
use netgen_rs::{NetgenParams, generate, write_dimacs};

let params = NetgenParams::from_slice(&[512, 10, 10, 2000, 5, 500, 1000, 3, 3, 20, 80, 50, 2000])
    .expect("valid params");
let result = generate(13502460, &params).unwrap();

// Write to stdout
let stdout = std::io::stdout();
write_dimacs(&mut stdout.lock(), 13502460, 1, &params, &result).unwrap();

// Or get as a string
let dimacs = netgen_rs::to_dimacs_string(13502460, 1, &params).unwrap();
```

## Provenance

The reference C code (in `netgen_original/`) is the **BCJL-patched version** of Norbert Schlenker's C implementation, with overflow fixes by Joseph Cheriyan that prevent infinite loops for networks with more than 2^15 nodes. This Rust port preserves the same overflow fixes using `f64` casts and removes the static `MAXNODES`/`MAXARCS` limits in favor of dynamic allocation.
