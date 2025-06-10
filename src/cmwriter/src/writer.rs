use ahash::{HashMap, HashMapExt};
use interop::TESTING;
use interop::{Vertex, TRIS_IN_CLUSTER};
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::{f32, usize};
use ultraviolet::Vec3;

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
struct Triangle {
    idx: usize,
    taken_by: usize,
    anchor: Vec3,
    // edge1, edge2, implicit_edge
    connections: [usize; 3],
}

struct AlgoMesh {
    pub triangles: Vec<Triangle>,
}

struct AlgoMeshSubset<'a> {
    // pub start: usize,
    // pub len: usize,
    pub idx_list: &'a mut [usize],
    // _phantom: PhantomCovariantLifetime<'a>,
}

// TODO add a tri trading step where clusters trade triangles
pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    // let mut maximum = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    // let mut minimum = Vec3::new(f32::MAX, f32::MAX, f32::MAX);

    // construct algo mesh
    let mut algo_mesh = AlgoMesh {
        triangles: {
            mesh.indices
                .chunks(3)
                .enumerate()
                .map(|(idx, chunk)| {
                    let vert0 = mesh.vertices[chunk[0] as usize];
                    let vert1 = mesh.vertices[chunk[1] as usize];
                    let vert2 = mesh.vertices[chunk[2] as usize];

                    let geo_mean = Vec3::new(
                        (vert0.position.x + vert1.position.x + vert2.position.x) / 3.0,
                        (vert0.position.y + vert1.position.y + vert2.position.y) / 3.0,
                        (vert0.position.z + vert1.position.z + vert2.position.z) / 3.0,
                    );

                    // for vert in [vert0, vert1, vert2] {
                    //     for (idx, val) in vert.position.as_array().iter().enumerate() {
                    //         if val < &minimum.as_array()[idx] {
                    //             minimum.as_mut_array()[idx] = *val;
                    //         }

                    //         if val > &maximum.as_array()[idx] {
                    //             maximum.as_mut_array()[idx] = *val;
                    //         }
                    //     }
                    // }

                    Triangle {
                        idx: idx,
                        taken_by: usize::MAX,
                        anchor: geo_mean,
                        connections: [usize::MAX, usize::MAX, usize::MAX],
                    }
                })
                .collect()
        },
    };
    println!("Done calcing the geo means.");

    // if two tris share an edge, they will be in opposite order because of triangle winding order
    // if it's not, undefined behavior, we don't support that
    let mut adjacency_graph = HashMap::with_capacity(mesh.vertices.len());

    // Fill connections in algo_mesh triangles
    {
        let mut run_count = 0;
        for tri_idx in 0..algo_mesh.triangles.len() {
            let tri = &algo_mesh.triangles[tri_idx];

            let idx0 = mesh.indices[(tri.idx * 3 + 0) as usize];
            let idx1 = mesh.indices[(tri.idx * 3 + 1) as usize];
            let idx2 = mesh.indices[(tri.idx * 3 + 2) as usize];

            let v = adjacency_graph.insert([idx0, idx1], tri_idx);
            assert_eq!(v, None);
            let v = adjacency_graph.insert([idx1, idx2], tri_idx);
            assert_eq!(v, None);
            let v = adjacency_graph.insert([idx2, idx0], tri_idx);
            assert_eq!(v, None);

            let adjacent_edges = [[idx1, idx0], [idx2, idx1], [idx0, idx2]];
            for (eidx, edge) in adjacent_edges.iter().enumerate() {
                if let Some(adjacent_idx) = adjacency_graph.get(edge) {
                    run_count += 1;
                    algo_mesh.triangles[tri_idx].connections[eidx] = *adjacent_idx;

                    let mut set_connection = false;
                    {
                        let adjacent = &mut algo_mesh.triangles[*adjacent_idx];
                        let tri_indices =
                            &mesh.indices[(adjacent.idx * 3)..=(adjacent.idx * 3 + 2)];
                        for (connection_idx, vert_idx) in tri_indices.iter().enumerate() {
                            if *vert_idx == edge[0] {
                                assert_eq!(adjacent.connections[connection_idx], usize::MAX);
                                adjacent.connections[connection_idx] = tri_idx;
                                set_connection = true;
                                break;
                            }
                        }
                    };

                    if !set_connection {
                        println!(""); // new line to make it easier to see the output
                        println!("Failed to set connections. Printing debug information.");
                        println!("Checked for {:?}", edge);
                        println!("Found {:?}", adjacent_idx);
                        println!(""); // new line to make it easier to see the output
                        let current = algo_mesh.triangles[tri_idx];
                        let tri_indices = &mesh.indices[(current.idx * 3)..=(current.idx * 3 + 2)];
                        println!("Current tri: {:?}", current);
                        println!("Verts of current: {:?}", tri_indices);
                        println!(""); // new line to make it easier to see the output
                        let adjacent = &mut algo_mesh.triangles[*adjacent_idx];
                        let tri_indices =
                            &mesh.indices[(adjacent.idx * 3)..=(adjacent.idx * 3 + 2)];
                        println!("Adjacent tri: {:?}", adjacent);
                        println!("Verts of adjacent: {:?}", tri_indices);
                        println!(""); // new line to make it easier to see the output
                        for (connection_idx, vert_idx) in tri_indices.iter().enumerate() {
                            println!("Testing tri {} against adj {}", edge[0], vert_idx);
                            if *vert_idx == edge[0] {
                                adjacent.connections[connection_idx] = tri_idx;
                                set_connection = true;
                                break;
                            }
                        }
                    }

                    #[cfg(debug_assertions)]
                    assert_eq!(set_connection, true);
                }
            }
        }
        println!("adjacency_graph.len {}", adjacency_graph.len());

        println!("Assigned: {}", run_count);
        println!("Verifying connections...");

        // THIS ONLY MATTERS FOR MANIFOLD MESHES/WATER TIGHT
        #[cfg(debug_assertions)]
        {
            println!("adjacency_graph.len {}", adjacency_graph.len());
            for (tri_idx, tri) in algo_mesh.triangles.iter().enumerate() {
                let idx0 = mesh.indices[(tri.idx * 3 + 0) as usize];
                let idx1 = mesh.indices[(tri.idx * 3 + 1) as usize];
                let idx2 = mesh.indices[(tri.idx * 3 + 2) as usize];

                for edge in [[idx0, idx1], [idx1, idx2], [idx2, idx0]] {
                    if let Some(my_idx) = adjacency_graph.get(&edge) {
                        assert_eq!(*my_idx, tri_idx);
                    } else {
                        panic!("Assigning connections failed! I don't exist!");
                    }
                }

                let adjacent_edges = [[idx1, idx0], [idx2, idx1], [idx0, idx2]];
                for (eidx, edge) in adjacent_edges.iter().enumerate() {
                    if let Some(adjacent_idx) = adjacency_graph.get(edge) {
                        assert_eq!(tri.connections[eidx], *adjacent_idx);
                    } else {
                        panic!("Connection: {} {:?} doesn't exist.", eidx, edge);
                    }
                }

                // this mesh might not have a fully contiguous surface...
                for (cn_idx, connection) in tri.connections.iter().enumerate() {
                    if *connection == usize::MAX {
                        panic!("Assigning connections failed!");
                    }
                }
            }
        };
    }

    // start splitting
    {
        let mut idx_list: Vec<usize> = (0..algo_mesh.triangles.len()).collect();
        println!("Subdividing...");
        let subsets = subdivide(
            &mesh.indices,
            &mesh.vertices,
            &mut algo_mesh.triangles,
            &mut idx_list,
        );
        println!("Done subdividing.");

        let mut not_equal = 0;
        for subset in &subsets {
            if subset.idx_list.len() < TRIS_IN_CLUSTER && subset.idx_list.len() > 0 {
                not_equal += 1;
            } else if subset.idx_list.len() > TRIS_IN_CLUSTER {
                println!("subset has: {} triangles", idx_list.len());
                panic!("Splitting failed!");
            }
        }
        assert!(not_equal <= 1);
    }

    // The entire mesh is one cluster. Find the two furthest points
    // split in two based on geo mean closer to which point then trade until they are roughly even number of triangles (optimize for one being multiple of TRIS_IN_CLUSTER)
    // choose two furthest, opposite points on edge (equidistant number of edges left or right to eachother, walk the edge to find them)
    // split the same way as before
    // repeat

    // Ideal datastruture for this:
    // - needs:
    //   - Get triangles/third point from an edge
    //   - easily expandable
    //   - mark triangles as taken/part of cluster
    //   - claimed vs unclaimed triangles
    //   - this is a graph, isn't it?

    // Algo:
    //   - Split mesh into clusters
    //   - Group 4 clusters into group
    //   - Simplify cluster groups, calc sq_error & anchor
    //   - Split simplified cluster groups into 2
    //   - Repeat from Group step (2) to create tree

    // -
    CTree { nodes, clusters }
}

fn subdivide<'a>(
    indices: &[u32],
    verts: &[Vertex],
    source: &'a mut [Triangle],
    idx_list: &'a mut [usize],
) -> Vec<AlgoMeshSubset<'a>> {
    let mut subsets = Vec::with_capacity(source.len() / TRIS_IN_CLUSTER + 1);

    let mut splittable = VecDeque::new();
    let source = source
        .iter_mut()
        .map(|tri| UnsafeCell::new(tri))
        .collect::<Vec<UnsafeCell<&mut Triangle>>>();
    splittable.push_back(AlgoMeshSubset { idx_list: idx_list });
    loop {
        if let Some(to_split) = splittable.pop_back() {
            let idx0 = subsets.len();
            subsets.push(AlgoMeshSubset { idx_list: &mut [] });
            let idx1 = subsets.len();
            subsets.push(AlgoMeshSubset { idx_list: &mut [] });
            let (split0, split1) = split(indices, verts, &source, to_split.idx_list, (idx0, idx1));

            // if split0.triangles.len() <= TRIS_IN_CLUSTER
            //     || split1.triangles.len() <= TRIS_IN_CLUSTER
            // {
            //     println!(
            //         "did it! {} {}",
            //         split0.triangles.len(),
            //         split1.triangles.len()
            //     );
            // }

            let len0 = split0.idx_list.len();
            if len0 <= TRIS_IN_CLUSTER {
                subsets[idx0] = split0;
            } else {
                splittable.push_back(split0);
            }

            let len1 = split1.idx_list.len();
            if len1 <= TRIS_IN_CLUSTER {
                subsets[idx1] = split1;
            } else {
                splittable.push_back(split1);
            }
        } else {
            break;
        }
    }

    subsets
}

fn split<'a>(
    indices: &[u32],
    verts: &[Vertex],
    source: &[UnsafeCell<&mut Triangle>],
    idx_list: &'a mut [usize],
    idxs: (usize, usize),
) -> (AlgoMeshSubset<'a>, AlgoMeshSubset<'a>) {
    let mut maximum = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut minimum = Vec3::new(f32::MAX, f32::MAX, f32::MAX);

    for tri in idx_list
        .iter()
        .map(|idx| unsafe { source[*idx].as_ref_unchecked() })
    {
        let vert0 = verts[indices[(tri.idx * 3 + 0) as usize] as usize];
        let vert1 = verts[indices[(tri.idx * 3 + 1) as usize] as usize];
        let vert2 = verts[indices[(tri.idx * 3 + 2) as usize] as usize];

        for vert in [vert0, vert1, vert2] {
            for (idx, val) in vert.position.as_array().iter().enumerate() {
                if val < &minimum.as_array()[idx] {
                    minimum.as_mut_array()[idx] = *val;
                }

                if val > &maximum.as_array()[idx] {
                    maximum.as_mut_array()[idx] = *val;
                }
            }
        }
    }

    let total_tris = idx_list.len();
    {
        let delta = maximum - minimum;
        let split_vert_pos_index = if delta.x > delta.y {
            if delta.x > delta.z {
                0
            } else {
                2
            }
        } else {
            if delta.y > delta.z {
                1
            } else {
                2
            }
        };

        unsafe {
            idx_list.sort_unstable_by(|a, b| {
                source[*a].as_ref_unchecked().anchor.as_array()[split_vert_pos_index].total_cmp(
                    &source[*b].as_ref_unchecked().anchor.as_array()[split_vert_pos_index],
                )
            });
        }
    };

    let split_size = {
        let ideal = idx_list.len() / 2;
        let leftover = ideal % TRIS_IN_CLUSTER;
        if leftover != 0 {
            ideal + TRIS_IN_CLUSTER - leftover
        } else {
            ideal
        }
    };

    {
        let mut src_to_split_idx = HashMap::with_capacity(idx_list.len());
        for (idx, value) in idx_list.iter().enumerate() {
            src_to_split_idx.insert(*value, idx);
        }
        let mut edge_0 = HashMap::new();
        // NEED TO ENSURE THE SPLITS ARE MANIFOLDS/CONTIGUOUS
        // TODO this breaks for non manifold tri pairs...
        // start from the lowest and go through, make sure everything is attached to it in some way
        // let mut settered = 0;
        let mut to_check = VecDeque::new();
        to_check.push_back(idx_list[0]);
        let mut consumed = 0;
        let mut last_resort = Vec::new();
        loop {
            if consumed == split_size {
                break;
            }

            if let Some(check_idx) = to_check.pop_back() {
                unsafe {
                    if source[check_idx].as_ref_unchecked().taken_by != idxs.0 {
                        source[check_idx].as_mut_unchecked().taken_by = idxs.0;
                        consumed += 1;
                    } else {
                        continue;
                    }

                    let tri = source[check_idx].as_ref_unchecked();
                    // connections is relative to the SOURCE SOURCE not the specific algo mesh...
                    for (conn_idx, connection) in tri.connections.iter().enumerate() {
                        if let Some(idx) = src_to_split_idx.get(connection) {
                            let connected_tri = source[*connection].as_ref_unchecked();
                            if connected_tri.taken_by != idxs.0 {
                                if *idx < split_size {
                                    to_check.push_back(*connection);
                                } else {
                                    last_resort.push(*idx);
                                }
                            }

                            // else {
                            //     // match conn_idx {
                            //     //     0 => {
                            //     //         let i0 = indices[tri.idx * 3 + 0];
                            //     //         let i1 = indices[tri.idx * 3 + 1];
                            //     //         edge_0.insert([i0, i1], check_idx);
                            //     //     }
                            //     //     1 => {
                            //     //         let i1 = indices[tri.idx * 3 + 1];
                            //     //         let i2 = indices[tri.idx * 3 + 2];
                            //     //         edge_0.insert([i1, i2], check_idx);
                            //     //     }
                            //     //     2 => {
                            //     //         let i2 = indices[tri.idx * 3 + 2];
                            //     //         let i0 = indices[tri.idx * 3 + 0];
                            //     //         edge_0.insert([i2, i0], check_idx);
                            //     //     }
                            //     //     _ => {
                            //     //         panic!("conn_idx is not 0, 1 or 2. This is compiler guaranteed. Either your PC is fucked, or a solar flare just flipped a bit.");
                            //     //     }
                            //     // }
                            // }
                        }
                    }
                }
            } else {
                break;
            }
        }

        // TODO make sure there are asserts but non manifold should be impossble based on how we chose them.
        // assert_eq!(settered, split_size);

        // let mut non_manifold0 = Vec::new();
        // let mut non_manifold1 = Vec::new();

        // for (idx, tri_idx) in idx_list.iter().enumerate() {
        //     let tri = unsafe { &source[*tri_idx].as_ref_unchecked() };
        //     if idx < split_size {
        //         if tri.taken_by != idxs.0 {
        //             non_manifold0.push(idx);
        //         }
        //     } else {
        //         if tri.taken_by == idxs.0 {
        //             non_manifold1.push(idx);
        //         }
        //     }
        // }

        // loop {
        //     if non_manifold0.len() > 0 && non_manifold1.len() > 0 {
        //         let tri0_split_idx = non_manifold0.pop().unwrap();
        //         let tri1_split_idx = non_manifold1.pop().unwrap();

        //         idx_list.swap(tri0_split_idx, tri1_split_idx);
        //         unsafe {
        //             source[idx_list[tri1_split_idx]].as_mut_unchecked().taken_by = idxs.1;
        //             source[idx_list[tri0_split_idx]].as_mut_unchecked().taken_by = idxs.0;
        //         };
        //     // } else if non_manifold0.len() > 0 {
        //     //     let tri0_split_idx = non_manifold0.pop().unwrap();
        //     //     let tri1_split_idx = dskfjsld;

        //     //     idx_list.swap(tri0_split_idx, tri1_split_idx);
        //     //     // find an edge tri in cluster 0
        //     //     // swap it with the non manifold here.
        //     // } else if non_manifold1.len() > 0 {
        //     //     let tri1_split_idx = non_manifold1.pop().unwrap();
        //     //     let tri0_split_idx = dskfjsld;

        //     //     idx_list.swap(tri0_split_idx, tri1_split_idx);
        //     //     // find an edge tri in cluster 1
        //     //     // swap it with the non manifold here.
        //     } else {
        //         break;
        //     }
        // }

        // assert_ne!(non_manifold0.len(), split_size);
        // assert_ne!(non_manifold1.len(), split_size);

        // if non_manifold0.len() > 0 || non_manifold1.len() > 0 {
        //     println!(
        //         "Non Manifold Tris: {} vs {}",
        //         non_manifold0.len(),
        //         non_manifold1.len()
        //     );
        // }
        // TODO finish non_manifold shit here
    };

    {
        let (split_tris0, split_tris1) = idx_list.split_at_mut(split_size);

        for tri_idx in split_tris0.iter() {
            let tri = &source[*tri_idx];
            unsafe {
                tri.as_mut_unchecked().taken_by = idxs.0;
            };
        }

        for tri_idx in split_tris1.iter() {
            let tri = &source[*tri_idx];
            unsafe {
                tri.as_mut_unchecked().taken_by = idxs.1;
            };
        }

        let split0 = AlgoMeshSubset {
            idx_list: split_tris0,
        };
        let split1 = AlgoMeshSubset {
            idx_list: split_tris1,
        };

        assert_eq!(split0.idx_list.as_ref().len(), split_size);
        assert_eq!(split1.idx_list.as_ref().len(), total_tris - split_size);

        (split0, split1)
    }

    // swap triangle positions in the vec so it can be split evenly
    // algo_mesh.triangles.swap(a, b);
    // algo_mesh.triangles.swap_unchecked(a, b);
}
