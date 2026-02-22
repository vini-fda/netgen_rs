//! NETGEN, a classic min-cost flow / assignment / max-flow generator.
//!
//! Produces DIMACS-formatted flow problem instances (assignment, max flow, or
//! min-cost flow) matching the original NETGEN (Klingman, Napier, Stutz, 1974)
//! implementation, including the BCJL overflow fixes.
//!
//! This crate is a pure-Rust rewrite of the
//! [reference C implementation](http://archive.dimacs.rutgers.edu/pub/netflow/generators/network/netgen/)
//! shipped with the DIMACS network flow challenge.
//!
//! # Usage
//!
//! ## Quick start
//!
//! ```rust
//! use netgen_rs::{generate, NetgenParams};
//!
//! let params = NetgenParams {
//!     nodes: 512,
//!     sources: 10,
//!     sinks: 10,
//!     density: 2000,
//!     mincost: 5,
//!     maxcost: 500,
//!     supply: 1000,
//!     tsources: 3,
//!     tsinks: 3,
//!     hicost_pct: 20,
//!     capacitated_pct: 80,
//!     mincap: 50,
//!     maxcap: 2000,
//! };
//! let result = generate(13502460, &params).unwrap();
//! println!("Generated {} arcs", result.arcs.len());
//! ```
//!
//! ## Writing DIMACS output
//!
//! Use [`write_dimacs`] to stream into any `io::Write`, or
//! [`to_dimacs_string`] to collect into a string.
//!
//! ```rust
//! use netgen_rs::{generate, write_dimacs, NetgenParams};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let params = NetgenParams::from_slice(&[
//!     512, 10, 10, 2000, 5, 500, 1000, 3, 3, 20, 80, 50, 2000,
//! ])?;
//! let dimacs = netgen_rs::to_dimacs_string(13502460, 1, &params)?;
//! println!("{}", dimacs.lines().next().unwrap());
//!
//! let result = generate(13502460, &params)?;
//! let stdout = std::io::stdout();
//! write_dimacs(&mut stdout.lock(), 13502460, 1, &params, &result)?;
//! # Ok(()) }
//! ```
//!
//! The 13 integers mirror the original `parms[]` array (see
//! [`NetgenParams`]).

mod index_list;
mod netgen;
mod random;

use std::fmt;
use std::io::{self, Write};

/// Parameters for network generation.
///
/// All fields are validated at construction time. Use [`NetgenParams::new`] or
/// [`NetgenParams::from_slice`] to create an instance, or construct manually
/// and call [`NetgenParams::validate`].
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
    pub hicost_pct: i64,
    pub capacitated_pct: i64,
    pub mincap: i64,
    pub maxcap: i64,
}

impl NetgenParams {
    /// Create validated parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nodes: i64,
        sources: i64,
        sinks: i64,
        density: i64,
        mincost: i64,
        maxcost: i64,
        supply: i64,
        tsources: i64,
        tsinks: i64,
        hicost_pct: i64,
        capacitated_pct: i64,
        mincap: i64,
        maxcap: i64,
    ) -> Result<Self, ParamError> {
        let params = NetgenParams {
            nodes,
            sources,
            sinks,
            density,
            mincost,
            maxcost,
            supply,
            tsources,
            tsinks,
            hicost_pct,
            capacitated_pct,
            mincap,
            maxcap,
        };
        params.validate()?;
        Ok(params)
    }

    /// Create from a slice of 13 values (matching the C `parms[]` array order).
    pub fn from_slice(parms: &[i64]) -> Result<Self, ParamError> {
        assert!(parms.len() >= 13);
        Self::new(
            parms[0], parms[1], parms[2], parms[3], parms[4], parms[5], parms[6], parms[7],
            parms[8], parms[9], parms[10], parms[11], parms[12],
        )
    }

    pub fn validate(&self) -> Result<(), ParamError> {
        if self.nodes <= 0 {
            return Err(ParamError::NonPositiveNodes);
        }
        if self.sources <= 0 {
            return Err(ParamError::NonPositiveSources);
        }
        if self.sinks <= 0 {
            return Err(ParamError::NonPositiveSinks);
        }
        if self.sources + self.sinks > self.nodes {
            return Err(ParamError::SourcesSinksExceedNodes);
        }
        if self.nodes > self.density {
            return Err(ParamError::DensityTooLow);
        }
        if self.mincost > self.maxcost {
            return Err(ParamError::MinCostExceedsMaxCost);
        }
        if self.supply < self.sources {
            return Err(ParamError::SupplyTooLow);
        }
        if self.tsources > self.sources {
            return Err(ParamError::TSourcesExceedSources);
        }
        if self.tsinks > self.sinks {
            return Err(ParamError::TSinksExceedSinks);
        }
        if self.hicost_pct < 0 || self.hicost_pct > 100 {
            return Err(ParamError::HiCostOutOfRange);
        }
        if self.capacitated_pct < 0 || self.capacitated_pct > 100 {
            return Err(ParamError::CapacitatedOutOfRange);
        }
        if self.mincap > self.maxcap {
            return Err(ParamError::MinCapExceedsMaxCap);
        }
        Ok(())
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

/// Specific parameter validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamError {
    NonPositiveNodes,
    NonPositiveSources,
    NonPositiveSinks,
    SourcesSinksExceedNodes,
    DensityTooLow,
    MinCostExceedsMaxCost,
    SupplyTooLow,
    TSourcesExceedSources,
    TSinksExceedSinks,
    HiCostOutOfRange,
    CapacitatedOutOfRange,
    MinCapExceedsMaxCap,
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamError::NonPositiveNodes => write!(f, "nodes must be positive"),
            ParamError::NonPositiveSources => write!(f, "sources must be positive"),
            ParamError::NonPositiveSinks => write!(f, "sinks must be positive"),
            ParamError::SourcesSinksExceedNodes => {
                write!(f, "sources + sinks must not exceed nodes")
            }
            ParamError::DensityTooLow => {
                write!(f, "density (arc count) must be at least nodes")
            }
            ParamError::MinCostExceedsMaxCost => write!(f, "mincost must not exceed maxcost"),
            ParamError::SupplyTooLow => write!(f, "supply must be at least sources"),
            ParamError::TSourcesExceedSources => {
                write!(f, "transshipment sources must not exceed sources")
            }
            ParamError::TSinksExceedSinks => {
                write!(f, "transshipment sinks must not exceed sinks")
            }
            ParamError::HiCostOutOfRange => write!(f, "hicost percentage must be 0..=100"),
            ParamError::CapacitatedOutOfRange => {
                write!(f, "capacitated percentage must be 0..=100")
            }
            ParamError::MinCapExceedsMaxCap => write!(f, "mincap must not exceed maxcap"),
        }
    }
}

impl std::error::Error for ParamError {}

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

/// Errors that may occur while running the generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetgenError {
    BadSeed,
    TooBig,
    BadParms,
    AllocationFailure,
}

impl fmt::Display for NetgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetgenError::BadSeed => write!(f, "seed must be positive"),
            NetgenError::TooBig => write!(f, "problem size exceeds limits"),
            NetgenError::BadParms => write!(f, "invalid parameters"),
            NetgenError::AllocationFailure => write!(f, "allocation failure"),
        }
    }
}

impl std::error::Error for NetgenError {}

/// Generate a network flow problem.
pub fn generate(seed: i64, params: &NetgenParams) -> Result<NetgenResult, NetgenError> {
    if seed <= 0 {
        return Err(NetgenError::BadSeed);
    }
    Ok(netgen::netgen(seed, params))
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
    writeln!(w, "c     With max cost:      {:10}%", params.hicost_pct)?;
    writeln!(
        w,
        "c     Capacitated:        {:10}%",
        params.capacitated_pct
    )?;
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
