#![feature(unsafe_cell_access)]

use foldhash::HashSet;
use interop::{Vertex, TRIS_IN_CLUSTER};
use ultraviolet::Vec3;
pub mod debug;
pub mod writer;

// Don't use this, it's SUPER SLOW for some reason
// pub(crate) fn count_intersections(e0: &HashSet<Edge>, tri: &Triangle) -> u8 {
//     let edges = [
//         [tri.idxs[0], tri.idxs[1]],
//         [tri.idxs[1], tri.idxs[2]],
//         [tri.idxs[2], tri.idxs[0]],
//     ];
//     let mut count = 0;
//     for edge in edges {
//         if e0.contains(&edge) {
//             count += 1
//         }
//     }

//     count
// }

pub(crate) fn insert(e0: &mut HashSet<Edge>, tri: &Triangle) {
    let edges = [
        [tri.idxs[1], tri.idxs[0]],
        [tri.idxs[2], tri.idxs[1]],
        [tri.idxs[0], tri.idxs[2]],
    ];
    for edge in edges {
        if e0.contains(&edge) {
            e0.remove(&edge);
        } else {
            e0.insert(edge);
        }
    }
}

pub fn intersect(e0: &mut HashSet<Edge>, other: &HashSet<Edge>) {
    // need to reverse the edge of one of them?
    // this only works if the edges have overlapping
    // sections, not just ven diagram overlap.
    // edges not points
    for edge in other {
        let key = {
            let mut temp = *edge;
            temp.reverse();
            temp
        };
        if e0.contains(&key) {
            e0.remove(&key);
        } else {
            e0.insert(*edge);
        }
    }
}

pub type Edge = [u32; 2];

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Cluster {
    pub mesh: Mesh,
    pub cut: Vec<u32>,
    pub anchor: Vec3,
}

pub struct CTree {
    pub nodes: Vec<CTreeNode>,
    pub clusters: Vec<Cluster>,
}

struct CTreeNode {
    // So, say an object that is 10 feet tall is 100 feet away. If I hold up a ruler 3 feet away, then the object in the
    // distance would correspond to about how many inches? => x/3 = 10/100
    pub max_sq_error: f32, // same input, same output
    pub anchor: Vec3,      // same input, same output
    pub parents: [usize; 2],
    pub children: [usize; 4],
}

impl CTree {
    pub fn new() -> CTree {
        CTree {
            nodes: Vec::new(),
            clusters: Vec::new(),
        }
    }

    pub fn from_raw(nodes: Vec<CTreeNode>, clusters: Vec<Cluster>) -> CTree {
        CTree { nodes, clusters }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Triangle {
    pub(crate) idx: usize,
    owned_by: usize,
    anchor: Vec3,
    // edge1, edge2, implicit_edge
    connections: [usize; 3],
    idxs: [u32; 3],
}

impl core::hash::Hash for Triangle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}

impl PartialEq for Triangle {
    fn eq(&self, other: &Self) -> bool {
        self.idx.eq(&other.idx)
    }
}

impl Eq for Triangle {}

impl PartialOrd for Triangle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}

impl Ord for Triangle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

struct AlgoMesh {
    pub triangles: Vec<Triangle>,
}

struct AlgoCluster {
    pub tri_idxs: Vec<usize>,
}
