use crate::debug::dump_clusters;
use crate::*;
use foldhash::{HashMap, HashMapExt};
use foldhash::{HashSet, HashSetExt};
use interop::TRIS_IN_CLUSTER;
use std::cell::UnsafeCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::usize;
use ultraviolet::{f32x8, Vec3, Vec3x8};

pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    // dump_raw(mesh, "RAW".to_string());

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

                    Triangle {
                        idx: idx,
                        owned_by: usize::MAX,
                        anchor: geo_mean,
                        connections: [usize::MAX, usize::MAX, usize::MAX],
                        idxs: [chunk[0], chunk[1], chunk[2]],
                    }
                })
                .collect()
        },
    };
    println!("Done calcing the geo means.");

    // if two tris share an edge, they will be in opposite order because of triangle winding order
    // if it's not, undefined behavior, we don't support that
    let mut adjacency_graph = HashMap::<Edge, usize>::with_capacity(mesh.vertices.len());

    // Fill connections in algo_mesh triangles
    {
        let mut run_count = 0;
        for tri_idx in 0..algo_mesh.triangles.len() {
            let tri = &algo_mesh.triangles[tri_idx];

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
    }

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
        let source = algo_mesh
            .triangles
            .iter_mut()
            .map(|tri| UnsafeCell::new(tri))
            .collect::<Vec<UnsafeCell<&mut Triangle>>>();
        let clusters = subdivide(&source, &adjacency_graph);
        println!("Done subdividing.");
        println!("Dumping.");
        dump_clusters(&mesh.vertices, &algo_mesh.triangles, &clusters, None);
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

struct PotentialTri {
    adjacent_edges: u8,
    dist: f32,
    idx: usize,
}

impl Eq for PotentialTri {}
impl PartialEq for PotentialTri {
    fn eq(&self, other: &Self) -> bool {
        let self_dist = self.dist * (3 - self.adjacent_edges) as f32;
        let other_dist = other.dist * (3 - other.adjacent_edges) as f32;
        self_dist.eq(&other_dist)
    }
}

impl PartialOrd for PotentialTri {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_dist = self.dist * (3 - self.adjacent_edges) as f32;
        let other_dist = other.dist * (3 - other.adjacent_edges) as f32;
        self_dist.partial_cmp(&other_dist)
    }
}

impl Ord for PotentialTri {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_dist = self.dist * (3 - self.adjacent_edges) as f32;
        let other_dist = other.dist * (3 - other.adjacent_edges) as f32;
        self_dist.total_cmp(&other_dist)
    }
}

// Algorithm options:
//  1. randomly toss the triangles into a heap or queue or whatever
//  2. take one out and see if it borders a cluster, if it does, check if it can be added
//  3. if it can't be added, start a new cluster from it.
//  4. if a triangle is bordered by 2 clusters, see if they can be merged.
fn subdivide(
    triangles: &[UnsafeCell<&mut Triangle>],
    adjacency_graph: &HashMap<Edge, usize>,
) -> Vec<AlgoCluster> {
    let mut clustered_tris = 0;
    let mut seed = {
        let seed = triangles
            .iter()
            .position(|tri| unsafe { tri.as_ref_unchecked().connections.contains(&usize::MAX) });

        if let Some(seed) = seed {
            seed
        } else {
            0
        }
    };
    // new idea, simultaneous growing, try to enforce hexagons/honeycomb tiling

    let add_new_potentials = |tri: &Triangle,
                              anchor: Vec3,
                              potential_tris: &mut BinaryHeap<Reverse<PotentialTri>>,
                              c_edge_cut: &HashSet<[u32; 2]>| {
        for connection in &tri.connections {
            if *connection != usize::MAX {
                let adjacent = unsafe { triangles[*connection].as_ref_unchecked() };
                if adjacent.owned_by == usize::MAX {
                    let dist = (adjacent.anchor - anchor).mag_sq();
                    let before = potential_tris.len();
                    potential_tris.retain(|pt| pt.0.idx != *connection); // is this necessary?
                    let after = potential_tris.len();

                    // let adjacent = count_intersections(&c_edge_cut, adjacent);
                    let adjacent = 1 + (before - after) as u8;
                    potential_tris.push(Reverse(PotentialTri {
                        adjacent_edges: adjacent,
                        dist,
                        idx: *connection,
                    }));
                }
            }
        }
    };

    // overall
    let mut clusters = Vec::with_capacity(triangles.len() / TRIS_IN_CLUSTER + 5);
    let mut edge_cut = HashSet::<Edge>::new();

    // cluster specific data types
    let mut potential_tris = BinaryHeap::with_capacity(TRIS_IN_CLUSTER);
    let mut c_edge_cut = HashSet::<Edge>::new();
    // choose this from the mesh edge
    loop {
        c_edge_cut.clear();
        let cluster_idx = clusters.len();

        // create a cluster
        let cluster = {
            let mut cluster = AlgoCluster {
                tri_idx_list: [usize::MAX; TRIS_IN_CLUSTER],
            };

            c_edge_cut.clear();
            potential_tris.clear();
            potential_tris.push(Reverse(PotentialTri {
                adjacent_edges: 1,
                dist: 0.0,
                idx: seed,
            }));

            let anchor = unsafe { triangles[seed].as_ref_unchecked().anchor };
            for tri_idx in 0..TRIS_IN_CLUSTER {
                if let Some(lowest) = potential_tris.pop() {
                    let lowest = lowest.0;
                    let low_tri = unsafe { triangles[lowest.idx].as_mut_unchecked() };
                    low_tri.owned_by = cluster_idx;
                    clustered_tris += 1;
                    add_new_potentials(low_tri, anchor, &mut potential_tris, &c_edge_cut);
                    insert(&mut c_edge_cut, low_tri);
                    cluster.tri_idx_list[tri_idx] = lowest.idx;
                } else {
                    // this probably isn't a panic, just a degen cluster
                    // panic!("There are no more border tris! Current idx: {} Overall Clustered: {} Currently: {}", cluster_idx, clustered_tris, tri_idx);
                    println!(
                        "Created a degen cluster idx {} with {} tris.",
                        cluster_idx, tri_idx
                    );
                    break;
                }
            }

            cluster
        };

        clusters.push(cluster);
        intersect(&mut edge_cut, &c_edge_cut);

        // find a new seed
        seed = {
            let mut seed_idx = 0;
            for edge in &edge_cut {
                if let Some(tri_idx) = adjacency_graph.get(edge) {
                    let tri = unsafe { triangles[*tri_idx].as_ref_unchecked() };
                    assert_eq!(tri.owned_by, usize::MAX);
                    // if this fails, something is wrong. None of the triangles
                    // outside of the edge cut should be clustered
                    let mut neighboring_clusters = 0;
                    for connection in &tri.connections {
                        if *connection != usize::MAX {
                            let tri = unsafe { triangles[*connection].as_ref_unchecked() };
                            if tri.owned_by != usize::MAX {
                                neighboring_clusters += 1;
                            }
                        }
                    }
                    if neighboring_clusters == 2 {
                        seed_idx = *tri_idx;
                        break;
                    }
                    // this is fallback if none of the tris have 2 overlapping edges
                    seed_idx = *tri_idx;

                    // let intersections = count_intersections(&edge_cut, tri);
                    // if intersections == 2 {
                    //     seed_idx = *tri_idx;
                    //     break;
                    // }

                    // // this is fallback if none of the tris have 2 overlapping edges
                    // seed_idx = *tri_idx;
                }
            }

            seed_idx
        };

        if clustered_tris == triangles.len() {
            break;
        }
    }

    println!("Clusters: {}", clusters.len());

    // this is used kind of like a queue but average O(1) removals... is this needed?
    // tris = HashSet::<Triangle>::with_capacity/from()
    //
    // this is initialized with the non manifold edge of the loaded mesh
    // the Edge in here is ordered to correspond to the missing triangle
    // or, the unclustered tris. This is overall.
    // edge_cut = HashSet::<Edge>
    //
    // from current cluster edge, lowest distance to anchor point from
    // seed tri anchor point is the one we take, avoid using vertex list
    // to minimize cache size.
    //
    // on finalize, we recalculate the anchor for the whole cluster (or just keep a running total)
    //
    // creating an edge HashSet for a cluster should be trivial. new hash set, check if every tri
    // edge is in there, if not, add the reverse edge. if it is there, remove it.
    //
    // for seed seeking, check if any border triangle has two taken neighbors, if so, consume it.
    //
    // fix floating/split degenerates by aggressively merging and shifting. how am I supposed to
    // know the direction to get there? Use AStar to traverse the graph connection hierarchy?
    // Store a HashMap of <[cluster_idx0, cluster_idx1], num_shared_edges>? this is the weighted graph?
    //
    // for trading/stealing, optimize for exposed edges and anchor point. if triangle with two edges
    // bordering another, try to trade it. do this in two phases? anchor point, then exposed edge trades
    // where possible, ie they both have tris they want to get rid of on their border?

    // let edge = [u32; 2];
    // edge.reverse();

    // -> Vec<AlgoMeshSubset<'a>>
    // Old:

    // Triangle = [u32; 3];
    // Edge = [u32; 2];
    // Point = [f32; 3];

    // indices =>
    //     edges: HashMap<Edge, [u32; 2]>,
    //     tris: HashSet<Triangle>,
    //     _vertices: HashMap<u32, Vec<u32>>,
    //     (
    //         vertex_map: HashMap<u32, HashSet<usize>>,
    //         tri_list: Vec<Triangle>
    //     )
    // ;
    //
    // cluster_edge: Vec<Edge>
    clusters
}
