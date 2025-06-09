use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::{fs::File, io::prelude::*};

use crate::ctree::*;
use crate::mesh::*;

#[derive(Clone)]
pub struct Cluster {
    // TODO make this have an internal Mesh object
    // pub positions: Vec<f32>,
    // pub indices: Vec<u32>,
    pub mesh: Mesh,
    // -------------------------------------------
    pub cut: HashSet<Edge>,
    pub anchor: Point,
}

#[derive(Clone)]
pub struct TempCluster {
    // TODO make this have an internal Mesh object
    // pub positions: Vec<f32>,
    // pub indices: Vec<u32>,
    pub mesh: TempMesh,
    // -------------------------------------------
    pub cut: HashSet<Edge>,
    pub anchor: Point,
}

#[derive(Debug)]
pub struct SharedEdges {
    // sum_of_two_highest_shared_edges -> vec[0] + vec[1],
    pub id: u32,
    pub lod: u32,
    pub connections: Vec<(u32, i32)>, // shared_edge_cluster_id, number_of_shared_edges // keep this sorted?
}

// pub fn split_mesh_new(
//     mesh: &Mesh,
//     TRIS_IN_CLUSTER: usize,
// ) -> (Vec<Cluster>, HashMap<u32, Vec<(u32, i32)>>, CTree) {
//     //                            num triangles
//     // let num_origins = (mesh.indices.len() / 3) / TRIS_IN_CLUSTER; // TODO try equidistant surface points
//
//     let (edges, tris, _vertices) = generate_data_structures(&mesh.indices);
//     let mut clusters = Vec::<Cluster>::with_capacity(mesh.indices.len() * 3 / TRIS_IN_CLUSTER + 1);
//
//     // place a point at a random vertex on the mesh surface
//     // RG that point
//     // get 6 mostly equidistant points on the edge of that surface and put these into the seeds list
//     // grow each seed list independently at the same time
//     // ---- POSSIBLY for each triangle to add, evaluate if it is closer to a
//     // for each cluster, get the edge that is furthest from the adjacent clusters then the one that is closest to the middle of those two
//     // then also make seeds between those that are new, need to make 1 then 6 then 12 then 18 seeds (ish)
//
//     let mut overall_cluster_cut = HashSet::<Edge>::new();
//     let mut tris = tris.clone();
//     let mut edges = edges.clone();
//
//     let mut seed = {
//         let edge = edges.keys().next().unwrap();
//         new_tri(edge[0], edge[1], edges[edge][0])
//     };
//
//     (clusters, shared_edges, ctree)
// }

/*
TODO
 - try a multi-region-growing
    - choose (mesh.triangles.count / TRIS_IN_CLUSTER) unique random points on the mesh, grow each one individually testing the surrounding seed locations to see where to put the clusters
    - for every cluster that is larger than 128 see if there are adjacent clusters with less triangles and move some triangles to those or split the cluster if it is larger than TRIS_IN_CLUSTER * 2 or 3 etc
    - ensure all clusters are <= TRIS_IN_CLUSTER

 - try a splitting approach
    - split the mesh in half and then those halves repeatedly until you have clusters where each is <= TRIS_IN_CLUSTER

*/

// TODO pub(crate) instead of all pub
pub fn split_mesh(
    mesh: &Mesh,
    TRIS_IN_CLUSTER: usize,
) -> (HashMap<u32, Vec<(u32, i32)>>, TempCTree, HashSet<Edge>) {
    let (edges, tris, _vertices, (vertex_map, tri_list)) = generate_data_structures(&mesh.indices);

    // ! Dict<idx, tri_idx> tri_idx references vec of tris = [u32; 3]

    let mut mesh_edge = HashSet::new();
    for edge in &edges {
        if edge.1[0] == u32::MAX || edge.1[1] == u32::MAX {
            mesh_edge.insert(*edge.0);
        }
    }

    let mut clusters = Vec::with_capacity(mesh.indices.len() * 3 / TRIS_IN_CLUSTER + 1);

    let mut overall_cluster_cut = HashSet::<Edge>::new();
    let mut tris = tris.clone();
    let mut edges = edges.clone();
    let mut degenerates = HashSet::new();

    // randomly choose a seed
    let mut seed = {
        let edge = edges.keys().next().unwrap();
        new_tri(edge[0], edge[1], edges[edge][0])
    };
    let mut counter = 0;
    while !tris.is_empty() {
        println!("count: {} seed: {:?}", counter, seed);
        counter += 1;
        if !tris.remove(&seed) {
            let filtered = tris.iter().filter(|tri| { tri.iter().any(|v| { *v == seed[0] || *v == seed[1] || *v == seed[2] }) }).collect::<Vec::<&[u32; 3]>>();
            println!("seed doesn't exist in tris:[{:?}],\n seed: ({:?})", filtered, seed);
            // break 'cluster_generation;
            panic!("seed doesn't exist in 'tris'");
        }

        let mut tri = seed;

        let anchor = mesh.mid_point_3(tri[0], tri[1], tri[2]);

        let mut cur_cluster_idxs = Vec::with_capacity(TRIS_IN_CLUSTER * 3);

        let order_accurate_tri = get_order_accurate_tri(&vertex_map, &tri_list, &tri);
        cur_cluster_idxs.extend_from_slice(&order_accurate_tri);
        let mut cur_cut = HashSet::new();

        remove_tri(
            &tri,
            &mut tris,
            &mut edges,
            &mut overall_cluster_cut,
            &mut cur_cut,
        );

        for _ in 0..TRIS_IN_CLUSTER - 1 {
            let mut lowest: (f32, Triangle) = (f32::MAX, [0, 0, 0]);

            // ----get the adjacent triangles to the current cluster edge path
            // ----choose the triangle where the outlying vertex is closest to the anchor
            'next_tri: for key in cur_cut.iter() {
                if let Some(value) = edges.get(key) {
                    let dst0 = if value[0] != u32::MAX {
                        mesh.sqr_dist_w_custom(value[0], anchor)
                    } else {
                        0.0
                    };

                    let dst1 = if value[1] != u32::MAX {
                        mesh.sqr_dist_w_custom(value[1], anchor)
                    } else {
                        0.0
                    };

                    if dst0 == 0.0 && dst1 == 0.0 {
                        // println!("continuing cause dst are both 0.0");
                        continue 'next_tri;
                    }

                    if dst0 > dst1 {
                        if dst0 < lowest.0 {
                            lowest = (dst0, [key[0], key[1], value[0]]);
                        }
                    } else if dst1 > dst0 {
                        if dst1 < lowest.0 {
                            lowest = (dst1, [key[0], key[1], value[1]]);
                        }
                    } else {
                        panic!("equidistant adjacent triangles, this should not be possible (theoretically)");
                    }
                }
            }

            if lowest.0 == f32::MAX {
                // println!("no suitable triangle to remove was found");
                // TODO every time a degenerate cluster is made, delete an adjacent cluster and remake a cluster from the edge of the degenerate triangle???
                // would need to remove in a specific direction
                break;
            }

            tri = new_tri(lowest.1[0], lowest.1[1], lowest.1[2]);

            remove_tri(
                &tri,
                &mut tris,
                &mut edges,
                &mut overall_cluster_cut,
                &mut cur_cut,
            );

            let order_accurate_tri = get_order_accurate_tri(&vertex_map, &tri_list, &tri);
            cur_cluster_idxs.extend_from_slice(&order_accurate_tri);
            tris.remove(&tri);

            // ----remove the chosen triangle
            // ----add the two new edges to the the current cluster edge path
        }

        // remove all edges from cluster edge path
        for edge in &cur_cut {
            if !overall_cluster_cut.insert(*edge) {
                overall_cluster_cut.remove(edge);
            }
        }

        //                 if cur_cluster_tris.len() < TRIS_IN_CLUSTER {
        //                     // degenerate_clusters.push(clusters.len() - 1);
        //                     unwinds_queued += 1;

        //                     if tris.len() == 0 {
        //                         // break ;
        // might not need to do anything cause the while loop stops on this condition anyways so continuing will end the loop
        //                     }
        //                     continue;
        //                 }

        // we have completed a cluster
        // let (mesh_indices, mesh_positions, mesh_cut) =
        // simplify_indices_positions_cut(&cur_cluster_idxs, &mesh.positions, &cur_cut);

        let cluster_mesh = TempMesh {
            // positions: mesh_positions,
            indices: cur_cluster_idxs, //mesh_indices,
        };

        let anchor = cluster_mesh.calculate_anchor(&mesh.positions);

        // println!("cluster completed");

        if cluster_mesh.indices.len() / 3 != TRIS_IN_CLUSTER {
            degenerates.insert(clusters.len() as u32);
        }

        clusters.push(TempCluster {
            mesh: cluster_mesh,
            cut: cur_cut,
            anchor,
        });

        // let mut should_break = false;

        // calculate the next seed
        'seed_seeking: for edge in &overall_cluster_cut {
            if let Some(verts) = edges.get(edge) {
                let vert = if verts[0] == u32::MAX {
                    if verts[1] == u32::MAX {
                        panic!("Both verts were u32::MAX. This should not be possible.");
                        // break 'cluster_generation;
                    }
                    verts[1]
                } else {
                    verts[0]
                };

                // choose one with only one edge remaining in the edges if it exists (ie has two edges in the overall cluster cut/edge_path)
                seed = new_tri(edge[0], edge[1], vert);
                // println!("FOUND ONE");

                let edge0 = new_edge(edge[0], vert);
                let edge1 = new_edge(edge[1], vert);
                // check if any of the other edges of the triangle are in the cut
                if overall_cluster_cut.contains(&edge0) || overall_cluster_cut.contains(&edge1) {
                    // we found an ideal seed triangle to use
                    break 'seed_seeking;
                }
            } else {
                println!("something seems to be weird here");
                println!("couldnt find the edge: {:?}", edge);
                // println!("in edges: {:?}", edges);

                continue;
                // should_break = true;
                // break;
            }
            // the seed is ok, keep looking to be sure
        }

        // under debug flag we can check for orphaned triangles and throw an error (or not in debug and add them to an adjacent cluster migrating triangles to maintain constant cluster size)

        // println!("Cluster number {} made with {} triangles", clusters.len(), clusters.last().unwrap().tris.len());

        // if should_break {
        //     break 'cluster_generation;
        // }
    }

    let mut shared_edges: HashMap<u32, Vec<(u32, i32)>> = HashMap::new();
    let mut ctree = TempCTree::from_clusters(clusters.clone());
    for id in 0..clusters.len() as u32 {
        let mut connections = Vec::with_capacity(10);
        for j in 0..clusters.len() as u32 {
            if id == j {
                continue;
            }

            let count = ctree.get(&id).cut.intersection(&ctree.get(&j).cut).count();
            if count > 0 {
                connections.push((j, count as i32));
            }
        }
        connections.sort_by(|a, b| {
            let cmp = b.1.cmp(&a.1);
            if cmp == Ordering::Equal {
                a.0.cmp(&b.0)
            } else {
                cmp
            }
        });
        shared_edges.insert(id, connections);

        // cut_length.push((id, ctree.get(&id).cut.len()));
    }

    if false {
        // adjacency graph
        //         for id in 0..clusters.len() as u32 {
        //             let mut connections = Vec::with_capacity(clusters.len());
        //             for j in 0..clusters.len() as u32 {
        //                 if id == j {
        //                     continue;
        //                 }
        //
        //                 let count = ctree.get(&id).cut.intersection(&ctree.get(&j).cut).count();
        //                 if count > 0 {
        //                     connections.push((j, count as i32));
        //                 }
        //             }
        //             shared_edges.insert(id, connections);
        //
        //             // cut_length.push((id, ctree.get(&id).cut.len()));
        //         }

        //         let mut sum = [0.0, 0.0, 0.0];
        //         // calculate the center of all the degenerates
        //         for didx in &degenerates {
        //             let degen = ctree.get(&didx);
        //             sum[0] += degen.anchor[0];
        //             sum[1] += degen.anchor[1];
        //             sum[2] += degen.anchor[2];
        //         }
        //         let center = [
        //             sum[0] / degenerates.len() as f32,
        //             sum[1] / degenerates.len() as f32,
        //             sum[2] / degenerates.len() as f32,
        //         ];
        //
        //         // for each degenerate, find the adjacent that is closest to the center that is not in the vec of previous adjacents visited
        //         for didx in &degenerates {
        //             // let adj =
        //         }
    };

    // iterative Monte-Carlo fix the meshes
    if false {
        // vec<length of cluster cut in vertices>
        // cluster adjacency graph
        // let mut cut_length = Vec::with_capacity(clusters.len());

        // ctree = CTree::from_clusters(clusters.clone());
        // let clusters: Vec<u32> = (0..clusters.len() as u32).collect();

        // list of tuples[3], index, index and number of shared edges
        // sort the list based on number of shared edges
        //         for id in 0..clusters.len() as u32 {
        //             let mut connections = Vec::with_capacity(clusters.len());
        //             for j in 0..clusters.len() as u32 {
        //                 if id == j {
        //                     continue;
        //                 }
        //
        //                 let count = ctree.get(&id).cut.intersection(&ctree.get(&j).cut).count();
        //                 if count > 0 {
        //                     connections.push((j, count as i32));
        //                 }
        //             }
        //             shared_edges.insert(id, connections);
        //
        //             // cut_length.push((id, ctree.get(&id).cut.len()));
        //         }

        // cut_length.sort_by(|a, b| a.1.cmp(&b.1));

        //         while let Some(popped) = cut_length.pop() {
        //             if popped.1 as f32 <= cut_length.get(0).unwrap().1 as f32 * 1.1 {
        //                 break;
        //             }
        //
        //             // get the triangle in the cluster that is furthest from the anchor of the cluster
        //             let mut furthest = (None, 0.0);
        //             let cluster = ctree.get(&popped.0);
        //             for idx in (0..cluster.mesh.indices.len()).step_by(3) {
        //                 let centroid = cluster.mesh.mid_point_3(
        //                     cluster.mesh.indices[idx + 0],
        //                     cluster.mesh.indices[idx + 1],
        //                     cluster.mesh.indices[idx + 2],
        //                 );
        //                 let dist = cluster.mesh.sqr_dist_w_2custom(centroid, cluster.anchor);
        //                 if dist > furthest.1 {
        //                     furthest = (Some(idx), dist);
        //                 }
        //             }
        //
        //             if let Some(tri_idx) = furthest.0 {
        //                 let (edges, tris, vertices) = generate_data_structures(&cluster.mesh.indices);
        //                 let centroid = cluster.mesh.mid_point_3(
        //                     cluster.mesh.indices[tri_idx + 0],
        //                     cluster.mesh.indices[tri_idx + 1],
        //                     cluster.mesh.indices[tri_idx + 2],
        //                 );
        //
        //                 // get a list of adjacent clusters
        //                 let adjacent = shared_edges.get(&popped.0).unwrap();
        //                 let mut closest = (None, f32::MAX);
        //                 for adj in adjacent {
        //                     let dist = cluster
        //                         .mesh
        //                         .sqr_dist_w_2custom(ctree.get(&adj.0).anchor, centroid);
        //
        //                     if dist < closest.1 {
        //                         closest = (Some(adj.0), dist);
        //                     }
        //                 }
        //
        //                 let dist = cluster.mesh.sqr_dist_w_2custom(cluster.anchor, centroid);
        //                 if dist < closest.1 {
        //                     println!("There is nothing to change in this cluster...");
        //                 } else if let Some(closest_idx) = closest.0 {
        //                     let new_cluster = ctree.get(&closest_idx);
        //                     // make sure to use the indices in the new cluster because the positions should already exist
        //                     let x_idx = find_equivalent(
        //                         &cluster.mesh.positions,
        //                         &new_cluster.mesh.positions,
        //                         &(tri_idx as u32 + 0),
        //                     )
        //                     .unwrap();
        //                     let y_idx = find_equivalent(
        //                         &cluster.mesh.positions,
        //                         &new_cluster.mesh.positions,
        //                         &(tri_idx as u32 + 0),
        //                     )
        //                     .unwrap();
        //                     let z_idx = find_equivalent(
        //                         &cluster.mesh.positions,
        //                         &new_cluster.mesh.positions,
        //                         &(tri_idx as u32 + 0),
        //                     )
        //                     .unwrap();
        //                     // ensure that the triangle is on the border of the cluster we are adding to
        //
        //                     // TODO need to map the indices before I can check this
        //                     if new_cluster.cut.contains(&new_edge(x_idx, y_idx))
        //                         || new_cluster.cut.contains(&new_edge(x_idx, z_idx))
        //                         || new_cluster.cut.contains(&new_edge(y_idx, z_idx))
        //                     {
        //                         // move the triangle to the mesh in closest.0
        //                     } else {
        //                         panic!("triangel is not on the border of this adjacent cluster.");
        //                     }
        //                 }
        //             }
        //
        //             // move that triangle to an adjacent cluster that is nearest it from the anchor (check that the distance to that anchor is closer than the distance to this anchor)
        //             // get the list of clusters that are adjacent to both the stolen-from and stealer
        //             // reevaluate the distance to the adjacent clusters for every cluster and move triangles?
        //         }
        //

        println!("degenerates before: {}", degenerates.len());

        // TODO make the clusters here share the main mesh positions list until this is done, then I dont have to mess with equivalent points and duped verts in the verts list
        loop {
            let mut triangles_moved = 0;
            for idx in 0..clusters.len() as u32 {
                let adjacents = shared_edges.get(&idx).unwrap();
                let mut modified = Vec::with_capacity(adjacents.len());

                for tri_idx in (0..ctree.get(&idx).mesh.indices.len()).step_by(3).rev() {
                    let tri = [
                        ctree.get(&idx).mesh.indices[tri_idx + 0],
                        ctree.get(&idx).mesh.indices[tri_idx + 1],
                        ctree.get(&idx).mesh.indices[tri_idx + 2],
                    ];
                    let centroid = mid_point_3(&mesh.positions, tri[0], tri[1], tri[2]);
                    let dist_o = sqr_dist_w_2custom(ctree.get(&idx).anchor, centroid);

                    let mut lowest = (idx, dist_o);
                    for adj in adjacents {
                        let adj_clust = ctree.get(&adj.0);
                        let dist_a = sqr_dist_w_2custom(adj_clust.anchor, centroid);

                        if dist_a < lowest.1 {
                            lowest = (adj.0, dist_a);
                        }
                    }

                    if lowest.0 != idx {
                        let edge0 = new_edge(tri[0], tri[1]);
                        let edge1 = new_edge(tri[0], tri[2]);
                        let edge2 = new_edge(tri[1], tri[2]);

                        {
                            let cluster = ctree.get_mut(&idx);
                            cluster.mesh.indices.remove(tri_idx);
                            cluster.mesh.indices.remove(tri_idx);
                            cluster.mesh.indices.remove(tri_idx);

                            if cluster.mesh.indices.len() != TRIS_IN_CLUSTER * 3 {
                                degenerates.insert(lowest.0);
                            }

                            if !cluster.cut.insert(edge0) {
                                cluster.cut.remove(&edge0);
                            }

                            if !cluster.cut.insert(edge1) {
                                cluster.cut.remove(&edge1);
                            }

                            if !cluster.cut.insert(edge2) {
                                cluster.cut.remove(&edge2);
                            }
                        }

                        // move to the new cluster
                        {
                            let new_cluster = ctree.get_mut(&lowest.0);
                            new_cluster.mesh.indices.push(tri[0]);
                            new_cluster.mesh.indices.push(tri[1]);
                            new_cluster.mesh.indices.push(tri[2]);

                            if new_cluster.mesh.indices.len() != TRIS_IN_CLUSTER * 3 {
                                degenerates.insert(lowest.0);
                            }

                            if !new_cluster.cut.insert(edge0) {
                                new_cluster.cut.remove(&edge0);
                            }

                            if !new_cluster.cut.insert(edge1) {
                                new_cluster.cut.remove(&edge1);
                            }

                            if !new_cluster.cut.insert(edge2) {
                                new_cluster.cut.remove(&edge2);
                            }

                            modified.push(lowest.0);
                        };

                        triangles_moved += 1;
                    }
                }

                if triangles_moved > 0 {
                    ctree.get_mut(&idx).anchor =
                        ctree.get(&idx).mesh.calculate_anchor(&mesh.positions);
                    for m in modified {
                        let mod_clust = ctree.get_mut(&m);
                        mod_clust.anchor = mod_clust.mesh.calculate_anchor(&mesh.positions);
                    }
                }
            }

            println!("moved: {}", triangles_moved);
            if triangles_moved == 0 {
                break;
            }
        }

        let degenerates: HashSet<u32> = degenerates.into_iter().collect();
        println!("degenerates after: {}", degenerates.len());
    };

    write_clusters(&mesh.positions, &clusters);

    // let clusters = clusters
    //     .clone()
    //     .into_iter()
    //     .map(|e| {
    //         let (indices, positions, cut) =
    //             simplify_indices_positions_cut(&e.mesh.indices, &mesh.positions, &e.cut);
    //         Cluster {
    //             mesh: Mesh { positions, indices },
    //             cut,
    //             anchor: e.anchor,
    //         }
    //     })
    //     .collect();

    // clusters as a return is temporary for debug printing
    (shared_edges, ctree, mesh_edge) //ctree.to_official(&mesh.positions))
}

pub fn get_order_accurate_tri(
    vertex_map: &HashMap<u32, HashSet<usize>>,
    tri_list: &Vec<Triangle>,
    tri: &Triangle,
) -> Triangle {
    let mut order_accurate_tri_idxs = vertex_map[&tri[0]]
        .iter()
        .filter(|e| vertex_map[&tri[1]].contains(e))
        .filter(|e| vertex_map[&tri[2]].contains(e));
    tri_list[*order_accurate_tri_idxs.next().unwrap()]
}

fn remove_tri(
    tri: &Triangle,
    tris: &mut HashSet<Triangle>,
    edges: &mut HashMap<Edge, [u32; 2]>,
    overall_cluster_cut: &mut HashSet<Edge>,
    cur_cut: &mut HashSet<Edge>,
) {
    tris.remove(tri);

    let edge0 = new_edge(tri[0], tri[1]);
    let edge1 = new_edge(tri[0], tri[2]);
    let edge2 = new_edge(tri[1], tri[2]);

    let mut remove0 = false;
    let mut remove1 = false;
    let mut remove2 = false;

    if let Some(value) = edges.get_mut(&edge0) {
        // if overall_cluster_cut.contains(&edge0) {
        //     remove0 = false;
        // } else
        if value[0] == tri[2] {
            value[0] = u32::MAX;
            if value[1] == u32::MAX {
                remove0 = true;
            }
        } else if value[1] == tri[2] {
            value[1] = u32::MAX;
            if value[0] == u32::MAX {
                remove0 = true;
            }
        }

        *value = new_edge(value[0], value[1]);
    }

    if let Some(value) = edges.get_mut(&edge1) {
        // if overall_cluster_cut.contains(&edge1) {
        //     remove1 = false;
        // } else
        if value[0] == tri[1] {
            value[0] = u32::MAX;
            if value[1] == u32::MAX {
                remove1 = true;
            }
        } else if value[1] == tri[1] {
            value[1] = u32::MAX;
            if value[0] == u32::MAX {
                remove1 = true;
            }
        }

        *value = new_edge(value[0], value[1]);
    }

    if let Some(value) = edges.get_mut(&edge2) {
        // if overall_cluster_cut.contains(&edge2) {
        //     remove2 = false;
        // } else
        if value[0] == tri[0] {
            value[0] = u32::MAX;
            if value[1] == u32::MAX {
                remove2 = true;
            }
        } else if value[1] == tri[0] {
            value[1] = u32::MAX;
            if value[0] == u32::MAX {
                remove2 = true;
            }
        }

        *value = new_edge(value[0], value[1]);
    }

    if remove0 {
        if !overall_cluster_cut.contains(&edge0) {
            cur_cut.remove(&edge0);
        } else {
            cur_cut.insert(edge0);
        }
        edges.remove(&edge0);
    } else {
        cur_cut.insert(edge0);
    }

    if remove1 {
        if !overall_cluster_cut.contains(&edge1) {
            cur_cut.remove(&edge1);
        } else {
            cur_cut.insert(edge1);
        }
        edges.remove(&edge1);
    } else {
        cur_cut.insert(edge1);
    }

    if remove2 {
        if !overall_cluster_cut.contains(&edge2) {
            cur_cut.remove(&edge2);
        } else {
            cur_cut.insert(edge2);
        }
        edges.remove(&edge2);
    } else {
        cur_cut.insert(edge2);
    }
}

fn write_clusters(positions: &Vec<f32>, clusters: &Vec<TempCluster>) {
    let mut f = File::create("clusters.obj").unwrap();
    f.write_all(b"mtllib bunny.mtl\no bun_zipper\n").unwrap();

    for pos in (0..positions.len()).step_by(3) {
        f.write_all(
            format!(
                "v {} {} {}\n",
                positions[pos + 0],
                positions[pos + 1],
                positions[pos + 2]
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all("\n".as_bytes()).unwrap();

    f.write_all(b"usemtl None\ns off\n").unwrap();

    for (idx, cluster) in clusters.iter().enumerate() {
        f.write_all(format!("o Cluster{}\n", idx + 1).as_bytes())
            .unwrap();
        for idx in (0..cluster.mesh.indices.len()).step_by(3) {
            f.write_all(
                format!(
                    "f {} {} {}\n",
                    cluster.mesh.indices[idx + 0] + 1,
                    cluster.mesh.indices[idx + 1] + 1,
                    cluster.mesh.indices[idx + 2] + 1
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all("\n".as_bytes()).unwrap();
    }

    f.flush().unwrap();
}
