# netgen-rs

This is a Rust translation of [NETGEN](http://archive.dimacs.rutgers.edu/pub/netflow/generators/network/netgen/), a program for generating large-scale capacitated assignment, transportation, and minimum-cost flow network problems, as described in:

> Klingman, Napier, and Stutz, "NETGEN: A program for generating large scale capacitated assignment, transportation, and minimum-cost flow network problems," *Management Science* 20, 814–820 (1974).

## Original C version

The reference C code used for this translation is the **BCJL-patched version** (found in `netgen_original/`), which includes bug fixes by Joseph Cheriyan (Bertsekas, Cheriyan, Jayakumar, Lam) applied to Norbert Schlenker's C implementation. The patches fix integer overflow issues in the original code that caused infinite loops when generating networks with more than 2^15 nodes, by casting intermediate products to `double` in critical computations (`sinks_per_source` calculation and the `pick_head` loop guard), and adding bounds checks on node indices.
