use ultraviolet::{Vec2, Vec3};

pub const TESTING: bool = false;
pub const TRIS_IN_CLUSTER: usize = 128;

#[derive(Clone)]
#[repr(C)]
pub struct Cluster {
    pub vertexes: Vec<Vertex>,
    pub idices: Vec<u32>,
}

#[repr(C)]
pub struct Node {
    pub offset: u32,
    pub children: [u32; 4],
}

// TODO this needs to be pulled from a common library or something.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    // pub normal: Vec3,
    // pub tex_coords: Vec2,
}
