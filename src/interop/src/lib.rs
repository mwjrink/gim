#[derive(Clone)]
#[repr(C)]
pub struct Cluster {
    pub pos: Vec<f32>,
    pub idx: Vec<u32>,
}

#[repr(C)]
pub struct Node {
    pub offset: u32,
    pub children: [u32; 4],
}

// TODO this needs to be pulled from a common library or something.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
// TODO should these be stored separately?
// probably not because we use them all in
// the same place, right? it is making
// blas construction slower in theory
// though
pub struct Vertex {
    pub position: [f32; 3],
    // pub color:  [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}
