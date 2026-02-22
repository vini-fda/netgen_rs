//! Core NETGEN network generator, faithfully ported from netgen.c.

use crate::index_list::IndexList;
use crate::random::Rng;
use crate::{Arc, NetgenError, NetgenParams, NetgenResult};

pub fn netgen(seed: i64, params: &NetgenParams) -> Result<NetgenResult, NetgenError> {
    if seed <= 0 {
        return Err(NetgenError::BadSeed);
    }
    params.validate()?;

    let nodes = params.nodes;
    let sources = params.sources;
    let sinks = params.sinks;
    let density = params.density;

    let mut rng = Rng::new(seed);
    let mut arcs: Vec<Arc> = Vec::new();
    let mut supply: Vec<i64> = vec![0; nodes as usize];

    let nodes_u = nodes as usize;
    let sources_u = sources as usize;
    let sinks_u = sinks as usize;
    let tsources = params.tsources;
    let tsinks = params.tsinks;
    let mut nodes_left: i64 = nodes - sinks + tsinks;

    // Check for assignment problem
    if (sources - tsources) + (sinks - tsinks) == nodes
        && (sources - tsources) == (sinks - tsinks)
        && sources == params.supply
    {
        create_assignment(params, &mut rng, &mut arcs, &mut supply, &mut nodes_left);
        return Ok(NetgenResult { arcs, supply });
    }

    create_supply(sources_u, params.supply, &mut rng, &mut supply);

    // Form skeleton
    let max_node = nodes_u;
    let alloc_size = nodes_u + density as usize + 2;
    let mut pred: Vec<usize> = vec![0; alloc_size];
    let mut head_arr: Vec<usize> = vec![0; alloc_size];
    let mut tail_arr: Vec<usize> = vec![0; alloc_size];

    #[allow(clippy::needless_range_loop)]
    for i in 1..=sources_u {
        pred[i] = i;
    }

    let mut handle = IndexList::new(sources_u + 1, max_node - sinks_u);
    let mut source: usize = 1;
    let transshipment = (nodes_u - sources_u - sinks_u) as i64;

    let threshold = (4 * transshipment + 9) / 10;
    let mut remaining = transshipment;
    while remaining > threshold {
        let node = handle.choose(rng.next(1, handle.size() as i64) as usize);
        pred[node] = pred[source];
        pred[source] = node;
        source += 1;
        if source > sources_u {
            source = 1;
        }
        remaining -= 1;
    }
    while remaining > 0 {
        let node = handle.choose(rng.next(1, handle.size() as i64) as usize);
        source = rng.next(1, sources) as usize;
        pred[node] = pred[source];
        pred[source] = node;
        remaining -= 1;
    }
    drop(handle);

    for source in 1..=sources_u {
        let mut sort_count: usize = 0;
        let mut node = pred[source];
        while node != source {
            sort_count += 1;
            head_arr[sort_count] = node;
            tail_arr[sort_count] = pred[node];
            node = pred[node];
        }

        let sinks_per_source: usize = if nodes_u - sources_u - sinks_u == 0 {
            (sinks_u / sources_u) + 1
        } else {
            // BCJL overflow fix: use f64
            ((2.0 * sort_count as f64 * sinks_u as f64) / (nodes_u - sources_u - sinks_u) as f64)
                as usize
        };
        let sinks_per_source = sinks_per_source.max(2).min(sinks_u);

        let mut sinks_vec: Vec<usize> = Vec::with_capacity(sinks_per_source);
        let mut handle = IndexList::new(max_node - sinks_u, max_node - 1);
        for _ in 0..sinks_per_source {
            sinks_vec.push(handle.choose(rng.next(1, handle.size() as i64) as usize));
        }

        if source == sources_u && handle.size() > 0 {
            while handle.size() > 0 {
                let j = handle.choose(1);
                if supply[j] == 0 {
                    sinks_vec.push(j);
                }
            }
        }
        drop(handle);

        let actual_sinks = sinks_vec.len();
        let chain_length = sort_count;
        let supply_per_sink = supply[source - 1] / actual_sinks as i64;
        let mut k = pred[source];

        for i in 0..actual_sinks {
            sort_count += 1;
            let partial_supply = rng.next(1, supply_per_sink);
            let j = rng.next(0, actual_sinks as i64 - 1) as usize;
            tail_arr[sort_count] = k;
            head_arr[sort_count] = sinks_vec[i] + 1;
            supply[sinks_vec[i]] -= partial_supply;
            supply[sinks_vec[j]] -= supply_per_sink - partial_supply;
            k = source;
            let mut steps = rng.next(1, chain_length as i64);
            while steps > 0 {
                k = pred[k];
                steps -= 1;
            }
        }
        supply[sinks_vec[0]] -= supply[source - 1] % actual_sinks as i64;

        sort_skeleton(&mut head_arr, &mut tail_arr, sort_count);
        tail_arr[sort_count + 1] = 0;

        let mut i = 1;
        while i <= sort_count {
            let mut handle = IndexList::new(sources_u - tsources as usize + 1, max_node);
            handle.remove(tail_arr[i]);
            let it = tail_arr[i];
            while it == tail_arr[i] {
                handle.remove(head_arr[i]);
                let mut cap = params.supply;
                if rng.next(1, 100) <= params.capacitated {
                    cap = supply[source - 1].max(params.mincap);
                }
                let mut cost = params.maxcost;
                if rng.next(1, 100) > params.hicost {
                    cost = rng.next(params.mincost, params.maxcost);
                }
                arcs.push(Arc {
                    from: it as u64,
                    to: head_arr[i] as u64,
                    cost,
                    capacity: cap,
                });
                i += 1;
            }
            pick_head(
                params,
                &mut handle,
                it,
                &mut nodes_left,
                &mut arcs,
                &mut rng,
            );
        }
    }

    // Add rubbish arcs out of transshipment sinks
    for i in (max_node - sinks_u + 1)..=(max_node - sinks_u + tsinks as usize) {
        let mut handle = IndexList::new(sources_u - tsources as usize + 1, max_node);
        handle.remove(i);
        pick_head(params, &mut handle, i, &mut nodes_left, &mut arcs, &mut rng);
    }

    Ok(NetgenResult { arcs, supply })
}

fn create_supply(sources: usize, total_supply: i64, rng: &mut Rng, supply: &mut [i64]) {
    let supply_per_source = total_supply / sources as i64;
    for i in 0..sources {
        let partial = rng.next(1, supply_per_source);
        supply[i] += partial;
        supply[rng.next(0, sources as i64 - 1) as usize] += supply_per_source - partial;
    }
    supply[rng.next(0, sources as i64 - 1) as usize] += total_supply % sources as i64;
}

fn create_assignment(
    params: &NetgenParams,
    rng: &mut Rng,
    arcs: &mut Vec<Arc>,
    supply: &mut [i64],
    nodes_left: &mut i64,
) {
    let nodes = params.nodes as usize;
    let sources = params.sources as usize;

    for s in supply.iter_mut().take(nodes / 2) {
        *s = 1;
    }
    for s in supply.iter_mut().take(nodes).skip(nodes / 2) {
        *s = -1;
    }

    let mut skeleton = IndexList::new(sources + 1, nodes);
    for source in 1..=nodes / 2 {
        let index = skeleton.choose(rng.next(1, skeleton.size() as i64) as usize);
        arcs.push(Arc {
            from: source as u64,
            to: index as u64,
            cost: rng.next(params.mincost, params.maxcost),
            capacity: 1,
        });
        let mut handle = IndexList::new(sources + 1, nodes);
        handle.remove(index);
        pick_head(params, &mut handle, source, nodes_left, arcs, rng);
    }
}

fn sort_skeleton(head: &mut [usize], tail: &mut [usize], sort_count: usize) {
    let mut m = sort_count;
    while {
        m /= 2;
        m != 0
    } {
        let k = sort_count - m;
        for j in 1..=k {
            let mut i = j as isize;
            while i >= 1 && tail[i as usize] > tail[(i as usize) + m] {
                let iu = i as usize;
                tail.swap(iu, iu + m);
                head.swap(iu, iu + m);
                i -= m as isize;
            }
        }
    }
}

fn pick_head(
    params: &NetgenParams,
    handle: &mut IndexList,
    desired_tail: usize,
    nodes_left: &mut i64,
    arcs: &mut Vec<Arc>,
    rng: &mut Rng,
) {
    let non_sources = params.nodes - params.sources + params.tsources;
    let remaining_arcs = params.density - arcs.len() as i64;

    *nodes_left -= 1;
    if (2 * *nodes_left) >= remaining_arcs {
        return;
    }

    let limit: i64;
    if (remaining_arcs + non_sources - handle.pseudo_size() as i64 - 1) / (*nodes_left + 1)
        >= non_sources - 1
    {
        limit = non_sources;
    } else {
        let upper_bound = 2 * (remaining_arcs / (*nodes_left + 1) - 1);
        loop {
            let mut l = rng.next(1, upper_bound);
            if *nodes_left == 0 {
                l = remaining_arcs;
            }
            // BCJL overflow fix: use f64 for the comparison
            let lhs = *nodes_left as f64 * (non_sources - 1) as f64;
            let rhs = (remaining_arcs - l) as f64;
            if lhs >= rhs {
                limit = l;
                break;
            }
        }
    };

    for _ in 0..limit {
        let index = handle.choose(rng.next(1, handle.pseudo_size() as i64) as usize);
        let mut cap = params.supply;
        if rng.next(1, 100) <= params.capacitated {
            cap = rng.next(params.mincap, params.maxcap);
        }

        // BCJL bounds check
        if index >= 1 && index <= params.nodes as usize {
            arcs.push(Arc {
                from: desired_tail as u64,
                to: index as u64,
                cost: rng.next(params.mincost, params.maxcost),
                capacity: cap,
            });
        }
    }
}
