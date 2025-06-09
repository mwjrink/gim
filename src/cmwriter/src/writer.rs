use interop::TESTING;
use interop::{Vertex, TRIS_IN_CLUSTER};
use ultraviolet::Vec3;

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone)]
pub struct Cluster {
    pub mesh: Mesh,
    pub cut: Vec<u32>,
    pub anchor: Vec3,
}

pub struct CTree {
    pub nodes: Vec<CTreeNode>,
    pub clusters: Vec<Cluster>,
}

pub struct CTreeNode {
    // So, say an object that is 10 feet tall is 100 feet away. If I hold up a ruler 3 feet away, then the object in the
    // distance would correspond to about how many inches? => x/3 = 10/100
    pub max_sq_error: f32, // same input, same output
    pub anchor: Vec3,      // same input, same output
    pub parents: [u32; 2],
    pub children: [u32; 4],
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

// TODO add a tri trading step where clusters trade triangles
pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    // Ideal datastruture for this:
    // - needs:
    //   - Get triangles/third point from an edge
    //   - easily expandable
    //   -

    // Algo:
    //   - Split mesh into clusters
    //   - Group 4 clusters into group
    //   - Simplify cluster groups, calc sq_error & anchor
    //   - Split simplified cluster groups into 2
    //   - Repeat from Group step (2) to create tree

    // -
    CTree { nodes, clusters }
}
