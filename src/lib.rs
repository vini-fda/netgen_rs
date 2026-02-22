//! NETGEN network flow problem generator (Rust port).
//!
//! A faithful translation of the classic NETGEN generator (Klingman, Napier, Stutz, 1974)
//! with BCJL overflow fixes.

mod index_list;
mod netgen;
mod random;

use std::fmt;
use std::io::{self, Write};

/// Parameters for network generation, replacing the raw `parms[]` array from the C code.
#[derive(Debug, Clone)]
pub struct NetgenParams {
    pub nodes: i64,
    pub sources: i64,
    pub sinks: i64,
    pub density: i64,
    pub mincost: i64,
    pub maxcost: i64,
    pub supply: i64,
    pub tsources: i64,
    pub tsinks: i64,
    pub hicost: i64,
    pub capacitated: i64,
    pub mincap: i64,
    pub maxcap: i64,
}

impl NetgenParams {
    /// Create from a slice of 13 values (matching the C `parms[]` array order).
    pub fn from_slice(parms: &[i64]) -> Self {
        assert!(parms.len() >= 13);
        NetgenParams {
            nodes: parms[0],
            sources: parms[1],
            sinks: parms[2],
            density: parms[3],
            mincost: parms[4],
            maxcost: parms[5],
            supply: parms[6],
            tsources: parms[7],
            tsinks: parms[8],
            hicost: parms[9],
            capacitated: parms[10],
            mincap: parms[11],
            maxcap: parms[12],
        }
    }

    /// Detect the problem type from the parameters.
    pub fn problem_type(&self) -> ProblemType {
        if (self.sources - self.tsources) + (self.sinks - self.tsinks) == self.nodes
            && (self.sources - self.tsources) == (self.sinks - self.tsinks)
            && self.sources == self.supply
        {
            ProblemType::Assignment
        } else if self.mincost == 1 && self.maxcost == 1 {
            ProblemType::MaxFlow
        } else {
            ProblemType::MinCostFlow
        }
    }
}

/// A single arc in the generated network.
#[derive(Debug, Clone)]
pub struct Arc {
    pub from: u64,
    pub to: u64,
    pub cost: i64,
    pub capacity: i64,
}

/// Result of network generation.
#[derive(Debug, Clone)]
pub struct NetgenResult {
    pub arcs: Vec<Arc>,
    /// Supply (positive) or demand (negative) at each node, 0-indexed.
    pub supply: Vec<i64>,
}

/// Problem type detected from parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemType {
    Assignment,
    MaxFlow,
    MinCostFlow,
}

/// Errors that can occur during generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetgenError {
    BadSeed,
    BadParms,
}

impl fmt::Display for NetgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetgenError::BadSeed => write!(f, "NETGEN requires a positive random seed"),
            NetgenError::BadParms => {
                write!(f, "Inconsistent parameter settings - check the input")
            }
        }
    }
}

impl std::error::Error for NetgenError {}

/// Generate a network flow problem.
pub fn generate(seed: i64, params: &NetgenParams) -> Result<NetgenResult, NetgenError> {
    netgen::netgen(seed, params)
}

/// Write the DIMACS-format header comments.
pub fn write_dimacs_header(
    w: &mut impl Write,
    seed: i64,
    problem: i64,
    params: &NetgenParams,
) -> io::Result<()> {
    writeln!(w, "c NETGEN flow network generator (C version)")?;
    writeln!(w, "c  Problem {:2} input parameters", problem)?;
    writeln!(w, "c  ---------------------------")?;
    writeln!(w, "c   Random seed:          {:10}", seed)?;
    writeln!(w, "c   Number of nodes:      {:10}", params.nodes)?;
    writeln!(w, "c   Source nodes:         {:10}", params.sources)?;
    writeln!(w, "c   Sink nodes:           {:10}", params.sinks)?;
    writeln!(w, "c   Number of arcs:       {:10}", params.density)?;
    writeln!(w, "c   Minimum arc cost:     {:10}", params.mincost)?;
    writeln!(w, "c   Maximum arc cost:     {:10}", params.maxcost)?;
    writeln!(w, "c   Total supply:         {:10}", params.supply)?;
    writeln!(w, "c   Transshipment -")?;
    writeln!(w, "c     Sources:            {:10}", params.tsources)?;
    writeln!(w, "c     Sinks:              {:10}", params.tsinks)?;
    writeln!(w, "c   Skeleton arcs -")?;
    writeln!(w, "c     With max cost:      {:10}%", params.hicost)?;
    writeln!(w, "c     Capacitated:        {:10}%", params.capacitated)?;
    writeln!(w, "c   Minimum arc capacity: {:10}", params.mincap)?;
    write!(w, "c   Maximum arc capacity: {:10}", params.maxcap)?;
    Ok(())
}

/// Write the DIMACS-format network data (problem line, node lines, arc lines).
pub fn write_dimacs_network(
    w: &mut impl Write,
    params: &NetgenParams,
    result: &NetgenResult,
) -> io::Result<()> {
    let num_arcs = result.arcs.len();
    let problem_type = params.problem_type();

    match problem_type {
        ProblemType::Assignment => {
            writeln!(w, "c")?;
            writeln!(w, "c  *** Assignment ***")?;
            writeln!(w, "c")?;
            writeln!(w, "p asn {} {}", params.nodes, num_arcs)?;
            for (i, &s) in result.supply.iter().enumerate() {
                if s > 0 {
                    writeln!(w, "n {}", i + 1)?;
                }
            }
            for arc in &result.arcs {
                writeln!(w, "a {} {} {}", arc.from, arc.to, arc.cost)?;
            }
        }
        ProblemType::MaxFlow => {
            writeln!(w, "c")?;
            writeln!(w, "c  *** Maximum flow ***")?;
            writeln!(w, "c")?;
            writeln!(w, "p max {} {}", params.nodes, num_arcs)?;
            for (i, &s) in result.supply.iter().enumerate() {
                if s > 0 {
                    writeln!(w, "n {} s", i + 1)?;
                } else if s < 0 {
                    writeln!(w, "n {} t", i + 1)?;
                }
            }
            for arc in &result.arcs {
                writeln!(w, "a {} {} {}", arc.from, arc.to, arc.capacity)?;
            }
        }
        ProblemType::MinCostFlow => {
            writeln!(w, "c")?;
            writeln!(w, "c  *** Minimum cost flow ***")?;
            writeln!(w, "c")?;
            writeln!(w, "p min {} {}", params.nodes, num_arcs)?;
            for (i, &s) in result.supply.iter().enumerate() {
                if s != 0 {
                    writeln!(w, "n {} {}", i + 1, s)?;
                }
            }
            for arc in &result.arcs {
                writeln!(
                    w,
                    "a {} {} {} {} {}",
                    arc.from, arc.to, 0, arc.capacity, arc.cost
                )?;
            }
        }
    }

    Ok(())
}

/// Write complete DIMACS output (header + network).
pub fn write_dimacs(
    w: &mut impl Write,
    seed: i64,
    problem: i64,
    params: &NetgenParams,
    result: &NetgenResult,
) -> io::Result<()> {
    write_dimacs_header(w, seed, problem, params)?;
    writeln!(w)?;
    write_dimacs_network(w, params, result)?;
    Ok(())
}

/// Generate and format as DIMACS string.
pub fn to_dimacs_string(
    seed: i64,
    problem: i64,
    params: &NetgenParams,
) -> Result<String, NetgenError> {
    let result = generate(seed, params)?;
    let mut buf = Vec::new();
    write_dimacs(&mut buf, seed, problem, params, &result).expect("writing to Vec should not fail");
    Ok(String::from_utf8(buf).expect("DIMACS output is ASCII"))
}
