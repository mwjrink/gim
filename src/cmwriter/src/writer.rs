use crate::debug::dump_clusters;
use crate::*;
use foldhash::{HashMap, HashMapExt};
use foldhash::{HashSet, HashSetExt};
use interop::TRIS_IN_CLUSTER;
use std::cell::UnsafeCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::future::IntoFuture;
use std::thread::sleep;
use std::time::Duration;
use std::usize;
use swait::FutureExt;
use ultraviolet::{f32x8, Vec3, Vec3x8};
use wrgpgpu::{include_wgsl, Bind, BindGroup, Device, ShaderArgs, StorageBufferBind};

pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    // dump_raw(mesh, "RAW".to_string());

    let triangles = mesh
        .indices
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

            UnsafeCell::new(Triangle {
                idx: idx,
                owned_by: usize::MAX,
                anchor: geo_mean,
                connections: [usize::MAX, usize::MAX, usize::MAX],
                idxs: [chunk[0], chunk[1], chunk[2]],
            })
        })
        .collect::<Vec<UnsafeCell<Triangle>>>();
    println!("Done calcing the geo means.");

    // if two tris share an edge, they will be in opposite order because of triangle winding order
    // if it's not, undefined behavior, we don't support that
    let mut adjacency_graph = build_adjacency_graph(&triangles, &mesh.indices);

    // start splitting
    {
        // let mut idx_list: Vec<usize> = (0..algo_mesh.triangles.len()).collect();
        // println!("Dumping...");
        // dump(
        //     &mesh.vertices,
        //     &mesh.indices,
        //     &algo_mesh.triangles,
        //     &idx_list,
        //     "Base".to_string(),
        // );
        // println!("dumped");

        println!("Subdividing...");
        let clusters = subdivide(&triangles, &adjacency_graph);
        println!("Done subdividing.");
        println!("Dumping.");
        // dump_clusters(&mesh.vertices, &triangles, &clusters, None);
        println!("Done dumping.");

        let mut not_equal = 0;
        // for (idx, subset) in subsets.iter().enumerate() {
        //     if subset.idx_list.len() < TRIS_IN_CLUSTER && subset.idx_list.len() > 0 {
        //         dump(
        //             &mesh.vertices,
        //             &mesh.indices,
        //             &algo_mesh.triangles,
        //             subset.idx_list,
        //             format!("final_degen_{}", idx).to_string(),
        //         );
        //         not_equal += 1;
        //     } else if subset.idx_list.len() > TRIS_IN_CLUSTER {
        //         println!("subset has: {} triangles", idx_list.len());
        //         panic!("Splitting failed!");
        //     } else {
        //         dump(
        //             &mesh.vertices,
        //             &mesh.indices,
        //             &algo_mesh.triangles,
        //             subset.idx_list,
        //             format!("final_{}", idx).to_string(),
        //         );
        //     }
        // }
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

struct ClusteredTri {
    idx: usize,
    cluster: usize,
    dist: f32,
}

impl Eq for ClusteredTri {}
impl PartialEq for ClusteredTri {
    fn eq(&self, other: &Self) -> bool {
        self.dist.eq(&other.dist)
    }
}

impl PartialOrd for ClusteredTri {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.dist.partial_cmp(&other.dist)
    }
}

impl Ord for ClusteredTri {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.dist.total_cmp(&other.dist)
    }
}

// Algorithm options:
//  1. randomly toss the triangles into a heap or queue or whatever
//  2. take one out and see if it borders a cluster, if it does, check if it can be added
//  3. if it can't be added, start a new cluster from it.
//  4. if a triangle is bordered by 2 clusters, see if they can be merged.
fn subdivide(
    triangles: &[UnsafeCell<Triangle>],
    adjacency_graph: &HashMap<Edge, usize>,
) -> Vec<AlgoCluster> {
    let clusters = Vec::<AlgoCluster>::with_capacity(triangles.len() / TRIS_IN_CLUSTER * 2);

    let mut seeds = Vec::with_capacity(triangles.len() / TRIS_IN_CLUSTER + 1);
    for tri_idx in (0..triangles.len()).step_by(TRIS_IN_CLUSTER) {
        seeds.push(tri_idx);
    }

    let device = Device::auto_high_performance().swait();
    // Create the shader...
    let shader = device.create_shader::<BindGroup<StorageBufferBind<[f32; 256]>>>(ShaderArgs {
        label: "Square",
        shader: include_wgsl!("simple.wgsl"),
        entrypoint: "square",
    });

    // Create and upload a buffer of data to work on...
    let mut n = 0.0;
    let buffer = StorageBufferBind::new_init(
        &device,
        [0.0; 256].map(|_| {
            n += 1.0;
            n
        }),
    );
    let bind_group = device.bind(&buffer);

    // Dispatch the compute shader, this assumes a workgroup size of (64, 1, 1) in the shader...
    device.dispatch(&shader, &bind_group, (1, 1, 1));

    // Wait until it is complete
    while !device.is_complete() {
        sleep(Duration::from_millis(1));
    }

    // Download the squared data
    let data = buffer.download(&device);

    println!("{:?}", data);

    // do something with the data...

    // calc on gpu nearest seed for each tri
    // return array of nearest seed?
    // now what?
    // move seeds and run again
    //   ideally I recenter or something like that... how do I get the center?
    // which direction and how much do I move them?

    // choose seed points
    // construct clusters from the seeds
    // run growth but check dist to other seeds to see which is closest
    // take the closest one
    // in 3d with a thin object, this could end up in 3 dimensions THIS IS NOT A PROBLEM IF GROWTH ONLY
    // have an adjacency with seeds?
    // Path find between them to create a graph?
    // only check against the nearest ones?
    // need a more efficient adj graph... or can I just use connections and ignore adj graph?
    //
    // What if... I ray traced the clusters?
    clusters
}

fn build_adjacency_graph(
    triangles: &[UnsafeCell<Triangle>],
    indices: &[u32],
) -> HashMap<Edge, usize> {
    let mut adjacency_graph = HashMap::<Edge, usize>::with_capacity(triangles.len() * 3);

    // Fill connections in algo_mesh triangles
    let mut run_count = 0;
    for tri_idx in 0..triangles.len() {
        let tri = &unsafe { triangles[tri_idx].as_ref_unchecked() };

        let [idx0, idx1, idx2] = tri.idxs;

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
                {
                    let mut_tri = unsafe { triangles[tri_idx].as_mut_unchecked() };
                    mut_tri.connections[eidx] = *adjacent_idx;
                }

                let mut set_connection = false;
                {
                    let adjacent = unsafe { triangles[*adjacent_idx].as_mut_unchecked() };
                    let tri_indices = &indices[(adjacent.idx * 3)..=(adjacent.idx * 3 + 2)];
                    for (connection_idx, vert_idx) in tri_indices.iter().enumerate() {
                        if *vert_idx == edge[0] {
                            assert_eq!(adjacent.connections[connection_idx], usize::MAX);
                            adjacent.connections[connection_idx] = tri_idx;
                            set_connection = true;
                            break;
                        }
                    }
                };

                // #[cfg(debug_assertions)]
                // if !set_connection {
                //     println!(""); // new line to make it easier to see the output
                //     println!("Failed to set connections. Printing debug information.");
                //     println!("Checked for {:?}", edge);
                //     println!("Found {:?}", adjacent_idx);
                //     println!(""); // new line to make it easier to see the output
                //     let current = triangles[tri_idx];
                //     let tri_indices = &indices[(current.idx * 3)..=(current.idx * 3 + 2)];
                //     println!("Current tri: {:?}", current);
                //     println!("Verts of current: {:?}", tri_indices);
                //     println!(""); // new line to make it easier to see the output
                //     let adjacent = &mut triangles[*adjacent_idx];
                //     let tri_indices = &indices[(adjacent.idx * 3)..=(adjacent.idx * 3 + 2)];
                //     println!("Adjacent tri: {:?}", adjacent);
                //     println!("Verts of adjacent: {:?}", tri_indices);
                //     println!(""); // new line to make it easier to see the output
                //     for (connection_idx, vert_idx) in tri_indices.iter().enumerate() {
                //         println!("Testing tri {} against adj {}", edge[0], vert_idx);
                //         if *vert_idx == edge[0] {
                //             // adjacent.connections[connection_idx] = tri_idx;
                //             set_connection = true;
                //             break;
                //         }
                //     }
                // }

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
        for (tri_idx, tri) in triangles.iter().enumerate() {
            let tri = unsafe { tri.as_ref_unchecked() };
            let [idx0, idx1, idx2] = tri.idxs;

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
                    // not all meshes are manifold
                    // panic!("Connection: {} {:?} doesn't exist.", eidx, edge);
                }
            }

            // this mesh might not have a fully contiguous surface...
            for (cn_idx, connection) in tri.connections.iter().enumerate() {
                if *connection == usize::MAX {
                    // not all meshes are manifold
                    // panic!("Assigning connections failed!");
                }
            }
        }
    };

    adjacency_graph
}
