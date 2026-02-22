//! Index list data structure, faithfully ported from index.c.
//!
//! An "index list" is an ascending sequence of positive integers that supports:
//! - `choose`: remove and return the k-th element
//! - `remove`: remove a specific integer
//! - `size`: actual count of remaining elements
//! - `pseudo_size`: size adjusted for failed remove attempts (preserves original NETGEN bug)
//!
//! Uses a flag array for small lists (≤ 100 elements) and a binary interval tree for larger ones.

const FLAG_LIMIT: usize = 100;

/// A node in the interval tree (large list implementation).
#[derive(Clone)]
struct IntervalNode {
    base: usize,
    count: usize,
    left_child: Option<usize>, // index into the nodes vec (left child; right child is +1)
}

/// Small list: flag array. Large list: interval tree.
enum ListImpl {
    Small {
        base: usize,
        flags: Vec<bool>, // true = removed
    },
    Large {
        nodes: Vec<IntervalNode>,
    },
}

pub struct IndexList {
    original_size: usize,
    index_size: usize,
    pseudo_size: usize,
    imp: ListImpl,
}

impl IndexList {
    /// Create a new index list containing integers from `from` through `to` inclusive.
    /// `from` must be >= 1 and `from <= to`.
    pub fn new(from: usize, to: usize) -> Self {
        assert!(from >= 1 && from <= to);
        let size = to - from + 1;

        let imp = if size <= FLAG_LIMIT {
            ListImpl::Small {
                base: from,
                flags: vec![false; size],
            }
        } else {
            let mut nodes = Vec::with_capacity(size);
            nodes.push(IntervalNode {
                base: from,
                count: size,
                left_child: None,
            });
            ListImpl::Large { nodes }
        };

        IndexList {
            original_size: size,
            index_size: size,
            pseudo_size: size,
            imp,
        }
    }

    /// Choose and remove the integer at the given 1-based position.
    /// Returns 0 if position is invalid.
    pub fn choose(&mut self, position: usize) -> usize {
        if position < 1 || position > self.index_size {
            return 0;
        }

        self.index_size -= 1;
        self.pseudo_size -= 1;

        match &mut self.imp {
            ListImpl::Small { base, flags } => {
                let mut remaining = position;
                for (i, flag) in flags.iter_mut().enumerate() {
                    if !*flag {
                        remaining -= 1;
                        if remaining == 0 {
                            *flag = true;
                            return *base + i;
                        }
                    }
                }
                unreachable!()
            }
            ListImpl::Large { nodes } => {
                let mut pos = position;
                let mut idx = 0; // root node

                // Walk down the tree
                while nodes[idx].left_child.is_some() {
                    nodes[idx].count -= 1;
                    let left = nodes[idx].left_child.unwrap();
                    if pos > nodes[left].count {
                        pos -= nodes[left].count;
                        idx = left + 1; // right child
                    } else {
                        idx = left;
                    }
                }

                nodes[idx].count -= 1;
                if pos == 1 {
                    // beginning of interval
                    let result = nodes[idx].base;
                    nodes[idx].base += 1;
                    result
                } else if pos > nodes[idx].count {
                    // end of interval
                    nodes[idx].base + nodes[idx].count
                } else {
                    // middle of interval - split it
                    let index = nodes[idx].base + pos - 1;
                    let new_left = nodes.len();
                    nodes.push(IntervalNode {
                        base: nodes[idx].base,
                        count: pos - 1,
                        left_child: None,
                    });
                    nodes.push(IntervalNode {
                        base: index + 1,
                        count: nodes[idx].count - (pos - 1),
                        left_child: None,
                    });
                    nodes[idx].left_child = Some(new_left);
                    index
                }
            }
        }
    }

    /// Remove a specific integer from the list. If it doesn't exist,
    /// the pseudo_size is still decremented (preserving original NETGEN behavior).
    pub fn remove(&mut self, index: usize) {
        self.pseudo_size -= 1;

        match &mut self.imp {
            ListImpl::Small { base, flags } => {
                if index < *base || index >= *base + self.original_size {
                    return;
                }
                let offset = index - *base;
                if !flags[offset] {
                    flags[offset] = true;
                    self.index_size -= 1;
                }
            }
            ListImpl::Large { nodes } => {
                // Walk down, decrementing counts along the way
                let mut idx = 0;
                let mut path = Vec::new();

                while nodes[idx].left_child.is_some() {
                    path.push(idx);
                    nodes[idx].count -= 1;
                    let left = nodes[idx].left_child.unwrap();
                    let right = left + 1;
                    if index < nodes[right].base {
                        idx = left;
                    } else {
                        idx = right;
                    }
                }

                // Check if index is actually in this interval
                if index < nodes[idx].base || index >= nodes[idx].base + nodes[idx].count {
                    // mistake - back out the decrements
                    for &p in &path {
                        nodes[p].count += 1;
                    }
                    return;
                }

                nodes[idx].count -= 1;
                if index == nodes[idx].base {
                    // beginning of interval
                    nodes[idx].base += 1;
                } else if index == nodes[idx].base + nodes[idx].count {
                    // end of interval - nothing extra to do
                } else {
                    // middle - split
                    let new_left = nodes.len();
                    nodes.push(IntervalNode {
                        base: nodes[idx].base,
                        count: index - nodes[idx].base,
                        left_child: None,
                    });
                    nodes.push(IntervalNode {
                        base: index + 1,
                        count: nodes[idx].count - (index - nodes[idx].base),
                        left_child: None,
                    });
                    nodes[idx].left_child = Some(new_left);
                }
                self.index_size -= 1;
            }
        }
    }

    pub fn size(&self) -> usize {
        self.index_size
    }

    pub fn pseudo_size(&self) -> usize {
        self.pseudo_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_list_basic() {
        let mut list = IndexList::new(1, 5);
        assert_eq!(list.size(), 5);
        // Choose 3rd element: should be 3
        assert_eq!(list.choose(3), 3);
        assert_eq!(list.size(), 4);
        // Choose 3rd element again: should be 4 (since 3 is gone)
        assert_eq!(list.choose(3), 4);
        assert_eq!(list.size(), 3);
    }

    #[test]
    fn small_list_remove() {
        let mut list = IndexList::new(1, 5);
        list.remove(3);
        assert_eq!(list.size(), 4);
        assert_eq!(list.choose(3), 4);
    }

    #[test]
    fn pseudo_size_on_missing_remove() {
        let mut list = IndexList::new(1, 5);
        list.remove(100); // doesn't exist
        assert_eq!(list.size(), 5);
        assert_eq!(list.pseudo_size(), 4); // decremented anyway
    }

    #[test]
    fn large_list_basic() {
        let mut list = IndexList::new(1, 200);
        assert_eq!(list.size(), 200);
        let v = list.choose(1);
        assert_eq!(v, 1);
        assert_eq!(list.size(), 199);
        let v = list.choose(199);
        assert_eq!(v, 200);
        assert_eq!(list.size(), 198);
    }

    #[test]
    fn large_list_remove() {
        let mut list = IndexList::new(1, 200);
        list.remove(100);
        assert_eq!(list.size(), 199);
        // Choose position that would have been 100
        let v = list.choose(99);
        assert_eq!(v, 99);
        let v = list.choose(99);
        assert_eq!(v, 101); // 100 was removed
    }

    #[test]
    fn choose_invalid_position() {
        let mut list = IndexList::new(1, 5);
        assert_eq!(list.choose(0), 0);
        assert_eq!(list.choose(6), 0);
    }
}
