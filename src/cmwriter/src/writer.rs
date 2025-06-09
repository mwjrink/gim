// create clusters of 128 triangles
// choose a seed triangle, choose the next vertex corresponding to an edge that is closest to the seed triangle

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

use crate::cluster::*;
use crate::ctree::*;
use crate::mesh::*;
use crate::utils::*;

const TESTING: bool = false;

const TRIS_IN_CLUSTER: usize = 128;

// TODO add a tri trading step where clusters trade triangles
pub fn write(mesh: &mut Mesh) -> CTree {
    // create data structure that can be referenced to find which indices are included in which triangles

    // let mut mesh = Mesh {
    //     indices: vec![],
    //     positions: mesh.positions.clone(),
    // };

    //
    // *
    // ! $Env:RUST_BACKTRACE=1
    // ?
    // TODO

    let (shared_edges, mut ctree, mesh_edge) = split_mesh(&mesh, TRIS_IN_CLUSTER);

    // let write_file = File::create("./logs/log").unwrap();
    // let mut writer = BufWriter::new(&write_file);
    // write!(writer, "shared_edges: \n{:?} \n", shared_edges).unwrap();

    println!("{} clusters generated", ctree.len());

    // panic!();

    // TODO this is called graph partitioning
    // TODO NANITE @Karris: Specifically, we optimize for the number of boundary edges and number of triangles per cluster
    //     use cluster tri trading here
    // !SECTION - Combine the clusters into a graph
    {
        let mut clusters: Vec<u32> = (0..ctree.len() as u32).collect();
        let mut shared_edges: Vec<SharedEdges> = shared_edges
            .into_iter()
            .map(|v| {
                // let mut connections = v.1;
                // connections.sort_by(|a, b| b.1.cmp(&a.1));
                SharedEdges {
                    id: v.0,
                    lod: 0,
                    connections: v.1,
                }
            })
            .collect();

        // find 4 clusters that share the most edges with eachother (closest anchors?)
        while clusters.len() > 3 {
            // TODO add weighting to clusters on the same LOD level as this? largest triangle area?
            // ^ this is where the multivariate optimization problem comes into play
            // grab the highest value of sum_of_two_highest_shared_edges and grab the highest index that is in both of the other connecting vecs
            shared_edges.sort_by(|element1, element2| {
                let cmp = element2.lod.cmp(&element1.lod);

                // ensure connections is sorted
                if cmp == Ordering::Equal {
                    let cmp2 = element1
                        .connections
                        .get(0)
                        .cmp(&element2.connections.get(0));

                    if cmp2 == Ordering::Equal {
                        element1.id.cmp(&element2.id)
                    } else {
                        cmp2
                    }
                } else {
                    cmp
                }
            });

            if shared_edges.is_empty() {
                println!(
                    "shared edges is empty but clusters has {} elements left.",
                    clusters.len()
                );
            }

            // ? TODO determine if any clusters are abandoned by combining these, include that one, ie ring abandons tip of ear on bunny
            // ? Is this, ^, actually an issue? it will just get picked up by a later cluster, it would be unoptimal but it wouldn't happen very often at all

            // get the cluster with the largest adjacency, add all the adjacents to a HashMap<u32, u32>
            // choose the highest value, choose it as the next cluster, add all the adjacents of that
            // cluster to the HashMap, if the value exists just add the values together. continue until
            // you have 4 values

            let mut adjacency_map = HashMap::new();

            // TODO the clusters are not adjacent, they are not connected
            let cluster0_id = shared_edges.last().unwrap().id;
            let cluster0_idx = shared_edges.len() - 1;
            let cluster0 = shared_edges.last().unwrap();
            for adj in &cluster0.connections {
                adjacency_map.insert(adj.0, adj.1);
            }

            // println!("cluster0 connections: {:?}", cluster0.connections);

            let cluster1_id = cluster0.connections.get(0).unwrap().0;
            let cluster1_idx = shared_edges
                .iter()
                .position(|e| e.id == cluster1_id)
                .unwrap();
            let cluster1 = shared_edges.get(cluster1_idx).unwrap();
            for adj in &cluster1.connections {
                let old = adjacency_map.insert(adj.0, adj.1);
                if let Some(value) = old {
                    *adjacency_map.get_mut(&adj.0).unwrap() += value;
                }
            }

            let mut adjacency_vec: Vec<(u32, i32)> =
                adjacency_map.iter().map(|v| (*v.0, *v.1)).collect();
            adjacency_vec.sort_by(|a, b| a.1.cmp(&b.1));

            let mut cluster2_id = u32::MAX;
            for adj in adjacency_vec.iter().rev() {
                if adj.0 != cluster0_id && adj.0 != cluster1_id {
                    cluster2_id = adj.0;
                    break;
                }
            }

            let cluster2_idx = shared_edges
                .iter()
                .position(|e| e.id == cluster2_id)
                .unwrap();
            let cluster2 = shared_edges.get(cluster2_idx).unwrap();
            for adj in &cluster2.connections {
                let old = adjacency_map.insert(adj.0, adj.1);
                if let Some(value) = old {
                    *adjacency_map.get_mut(&adj.0).unwrap() += value;
                }
            }

            let mut adjacency_vec: Vec<(u32, i32)> =
                adjacency_map.iter().map(|v| (*v.0, *v.1)).collect();
            adjacency_vec.sort_by(|a, b| a.1.cmp(&b.1));

            let mut cluster3_id = u32::MAX;
            for adj in adjacency_vec.iter().rev() {
                if adj.0 != cluster0_id && adj.0 != cluster1_id && adj.0 != cluster2_id {
                    cluster3_id = adj.0;
                    break;
                }
            }
            // println!("cluster3_id: {}", cluster3_id);
            let cluster3_idx = shared_edges
                .iter()
                .position(|e| e.id == cluster3_id)
                .unwrap();
            let cluster3 = shared_edges.get(cluster3_idx).unwrap();
            // };

            //             println!("cluster0 id: {}", cluster0_id);
            //             println!("cluster0 adjacency: {:?}", cluster0.connections);
            //             println!();
            //
            //             println!("cluster1 id: {}", cluster1_id);
            //             println!("cluster1 adjacency: {:?}", cluster1.connections);
            //             println!();
            //
            //             println!("cluster2 id: {}", cluster2_id);
            //             println!("cluster2 adjacency: {:?}", cluster2.connections);
            //             println!();
            //
            //             println!("cluster3 id: {}", cluster3_id);
            //             println!("cluster3 adjacency: {:?}", cluster3.connections);
            //             println!();

            // TODO new algorithm for choosing what to merge
            // sort the list by lod level with the a + b as a tie breaker?

            // combine the clusters into one mesh
            // let mut triangles = Vec::<Triangle>::with_capacity(
            //     clusters[cluster0_id].tris.len()
            //         + clusters[cluster1_id].tris.len()
            //         + clusters[cluster2_id].tris.len()
            //         + clusters[cluster3_id].tris.len(),
            // );
            // triangles.extend_from_slice(&clusters[cluster0_id].tris);
            // triangles.extend_from_slice(&clusters[cluster1_id].tris);
            // triangles.extend_from_slice(&clusters[cluster2_id].tris);
            // triangles.extend_from_slice(&clusters[cluster3_id].tris);

            // idx += clusters[cluster3_id].positions.len();

            // let border = clusters[starting_point.id].cut
            //     .symmetric_difference(&clusters[cluster1_id].cut).map(|value| { value.clone() }).collect::<HashSet<[u32; 2]>>()
            //     .symmetric_difference(&clusters[cluster2_id].cut).map(|value| { value.clone() }).collect::<HashSet<[u32; 2]>>()
            //     .symmetric_difference(&clusters[cluster3_id].cut).map(|value| { value.clone() }).collect::<HashSet<[u32; 2]>>();

            let mesh_indices = {
                // mesh_positions
                let mut indices = Vec::with_capacity(
                    ctree.get(&cluster0_id).mesh.indices.len()
                        + ctree.get(&cluster1_id).mesh.indices.len()
                        + ctree.get(&cluster2_id).mesh.indices.len()
                        + ctree.get(&cluster3_id).mesh.indices.len(),
                );

                indices.extend_from_slice(&ctree.get(&cluster0_id).mesh.indices);
                indices.extend_from_slice(&ctree.get(&cluster1_id).mesh.indices);
                indices.extend_from_slice(&ctree.get(&cluster2_id).mesh.indices);
                indices.extend_from_slice(&ctree.get(&cluster3_id).mesh.indices);
                //
                //                 let mut positions = Vec::with_capacity(
                //                     ctree.get(&cluster0_id).mesh.positions.len()
                //                         + ctree.get(&cluster1_id).mesh.positions.len()
                //                         + ctree.get(&cluster2_id).mesh.positions.len()
                //                         + ctree.get(&cluster3_id).mesh.positions.len(),
                //                 );
                //
                //                 // might be faster with a .extend_from_slice(clusters[cluster0_id].indices.map(|a| { a + idx })) etc
                //                 let mut idx = 0;
                //                 for i in &ctree.get(&cluster0_id).mesh.indices {
                //                     indices.push(i + idx);
                //                 }
                //                 positions.extend_from_slice(&ctree.get(&cluster0_id).mesh.positions);
                //                 idx += ctree.get(&cluster0_id).mesh.positions.len() as u32 / 3;
                //                 for i in &ctree.get(&cluster1_id).mesh.indices {
                //                     indices.push(i + idx);
                //                 }
                //                 positions.extend_from_slice(&ctree.get(&cluster1_id).mesh.positions);
                //                 idx += ctree.get(&cluster1_id).mesh.positions.len() as u32 / 3;
                //                 for i in &ctree.get(&cluster2_id).mesh.indices {
                //                     indices.push(i + idx);
                //                 }
                //                 positions.extend_from_slice(&ctree.get(&cluster2_id).mesh.positions);
                //                 idx += ctree.get(&cluster2_id).mesh.positions.len() as u32 / 3;
                //                 for i in &ctree.get(&cluster3_id).mesh.indices {
                //                     indices.push(i + idx);
                //                 }
                //                 positions.extend_from_slice(&ctree.get(&cluster3_id).mesh.positions);

                // remove_duplicated_vertices(&mut indices, &mut mesh.positions);

                // simplify_indices_positions(&indices, &mut mesh.positions)
                indices
            };

            // println!("max index: {}, positions len: {}", mesh_indices.iter().max().unwrap(), mesh_positions.len());

            let combined = TempMesh {
                // positions: mesh_positions,
                indices: mesh_indices,
            };

            // mesh simplification
            let cluster = simplify(
                &combined,
                &mut mesh.positions,
                &mesh_edge,
                (TRIS_IN_CLUSTER * 2) as u32,
            );

            //             println!("writing cluster0 id: {}", &cluster0_id);
            //             ctree
            //                 .get(&cluster0.id)
            //                 .mesh
            //                 .write_mesh_to_file("cluster0", &mut mesh.positions);
            //             println!("writing cluster1 id: {}", &cluster1_id);
            //             ctree
            //                 .get(&cluster1.id)
            //                 .mesh
            //                 .write_mesh_to_file("cluster1", &mut mesh.positions);
            //             println!("writing cluster2 id: {}", &cluster2_id);
            //             ctree
            //                 .get(&cluster2.id)
            //                 .mesh
            //                 .write_mesh_to_file("cluster2", &mut mesh.positions);
            //             println!("writing cluster3 id: {}", &cluster3_id);
            //             ctree
            //                 .get(&cluster3.id)
            //                 .mesh
            //                 .write_mesh_to_file("cluster3", &mut mesh.positions);

            // println!(
            //     "writing new_indices with {} tris",
            //     cluster.indices.len() / 3
            // );
            // write_mesh_to_file("new_indices", &cluster.positions, &cluster.indices);

            if cluster.indices.len() > TRIS_IN_CLUSTER * 3 * 2 {
                panic!("unable to simplify the cluster enough");
            }

            // calculate new cut
            // let (_, tris, _) = generate_data_structures(&cluster.indices);

            let combined_anchor = cluster.calculate_anchor(&mesh.positions);
            // split cluster into 2, choose any triangle on the border as a seed, same algorithm as above

            // TODO, some free floating triangles
            let mut furthest = ([0.0f32, 0.0, 0.0], 0.0 as f32);
            // let mut furthest_tri: Triangle = [0, 0, 0];
            for idx in (0..cluster.indices.len()).step_by(3) {
                let mid_point = mid_point_3(
                    &mesh.positions,
                    cluster.indices[idx + 0],
                    cluster.indices[idx + 1],
                    cluster.indices[idx + 2],
                );
                let dist = sqr_dist_w_2custom(
                    mid_point_3(
                        &mesh.positions,
                        cluster.indices[idx + 0],
                        cluster.indices[idx + 1],
                        cluster.indices[idx + 2],
                    ),
                    combined_anchor,
                );
                if dist > furthest.1 {
                    furthest = (mid_point, dist);
                    // furthest_tri = *tri;
                }
            }

            let mut cluster1_idxs = Vec::with_capacity(TRIS_IN_CLUSTER);
            let mut cluster1_edges = HashMap::<Edge, [u32; 2]>::with_capacity(TRIS_IN_CLUSTER * 3);

            let mut cluster2_idxs = Vec::with_capacity(TRIS_IN_CLUSTER);
            let mut cluster2_edges = HashMap::<Edge, [u32; 2]>::with_capacity(TRIS_IN_CLUSTER * 3);

            let mut sorted_tris = Vec::new();
            for idx in (0..cluster.indices.len()).step_by(3) {
                let mid_point = mid_point_3(
                    &mesh.positions,
                    cluster.indices[idx + 0],
                    cluster.indices[idx + 1],
                    cluster.indices[idx + 2],
                );
                let dist1 = sqr_dist_w_2custom(furthest.0, mid_point);

                sorted_tris.push((
                    dist1,
                    [
                        cluster.indices[idx + 0],
                        cluster.indices[idx + 1],
                        cluster.indices[idx + 2],
                    ],
                ));
            }
            sorted_tris.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            // TODO also have to make sure all these are connected, could have the two sides of an ear which would be closer but also disconnected
            // ? ^ is disconnected an issue at all? does it matter for rendering? it might make a difference for culling?
            for idx in 0..sorted_tris.len() {
                if idx < TRIS_IN_CLUSTER {
                    cluster1_idxs.push(sorted_tris[idx].1[0]);
                    cluster1_idxs.push(sorted_tris[idx].1[1]);
                    cluster1_idxs.push(sorted_tris[idx].1[2]);

                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[0], sorted_tris[idx].1[1]);
                        let value = cluster1_edges.insert(key, [sorted_tris[idx].1[2], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[2]
                                );
                            }
                            cluster1_edges.insert(key, [sorted_tris[idx].1[2], v[0]]);
                        }
                    }
                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[0], sorted_tris[idx].1[2]);
                        let value = cluster1_edges.insert(key, [sorted_tris[idx].1[1], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[1]
                                );
                            }
                            cluster1_edges.insert(key, [sorted_tris[idx].1[1], v[0]]);
                        }
                    }
                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[1], sorted_tris[idx].1[2]);
                        let value = cluster1_edges.insert(key, [sorted_tris[idx].1[0], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[0]
                                );
                            }
                            cluster1_edges.insert(key, [sorted_tris[idx].1[0], v[0]]);
                        }
                    }
                } else {
                    cluster2_idxs.push(sorted_tris[idx].1[0]);
                    cluster2_idxs.push(sorted_tris[idx].1[1]);
                    cluster2_idxs.push(sorted_tris[idx].1[2]);

                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[0], sorted_tris[idx].1[1]);
                        let value = cluster2_edges.insert(key, [sorted_tris[idx].1[2], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[2]
                                );
                            }
                            cluster2_edges.insert(key, [sorted_tris[idx].1[2], v[0]]);
                        }
                    }
                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[0], sorted_tris[idx].1[2]);
                        let value = cluster2_edges.insert(key, [sorted_tris[idx].1[1], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[1]
                                );
                            }
                            cluster2_edges.insert(key, [sorted_tris[idx].1[1], v[0]]);
                        }
                    }
                    {
                        let key: Edge = new_edge(sorted_tris[idx].1[1], sorted_tris[idx].1[2]);
                        let value = cluster2_edges.insert(key, [sorted_tris[idx].1[0], u32::MAX]);
                        if let Some(v) = value {
                            if v[1] != u32::MAX {
                                println!(
                                    "this edge has appeared more than twice: {:?} => {:?} + {}",
                                    key, v, sorted_tris[idx].1[0]
                                );
                            }
                            cluster2_edges.insert(key, [sorted_tris[idx].1[0], v[0]]);
                        }
                    }
                }
            }

            let combined1_anchor = cluster.calculate_anchor(&mesh.positions);
            let combined2_anchor = cluster.calculate_anchor(&mesh.positions);

            // calculate cut
            let combined1_cut = {
                let mut cut = HashSet::<Edge>::new();
                for kv in cluster1_edges {
                    if kv.1[0] == u32::MAX || kv.1[1] == u32::MAX {
                        cut.insert(kv.0);
                    }
                }

                cut
            };
            let combined2_cut = {
                let mut cut = HashSet::<Edge>::new();
                for kv in cluster2_edges {
                    if kv.1[0] == u32::MAX || kv.1[1] == u32::MAX {
                        cut.insert(kv.0);
                    }
                }

                cut
            };

            // let (indices, positions) =
            //     simplify_indices_positions(&cluster1_idxs, &cluster.positions.clone());
            let combined1 = TempCluster {
                mesh: TempMesh {
                    indices: cluster1_idxs,
                },
                cut: combined1_cut,
                anchor: combined1_anchor,
            };
            // let (indices, positions) =
            //     simplify_indices_positions(&cluster2_idxs, &cluster.positions.clone());
            let combined2 = TempCluster {
                mesh: TempMesh {
                    indices: cluster2_idxs,
                },
                cut: combined2_cut,
                anchor: combined2_anchor,
            };

            let idx1 = ctree.insert(
                combined1,
                &[u32::MAX, u32::MAX],
                &[cluster0_id, cluster1_id, cluster2_id, cluster3_id],
            );
            clusters.push(idx1);

            let idx2 = ctree.insert(
                combined2,
                &[u32::MAX, u32::MAX],
                &[cluster0_id, cluster1_id, cluster2_id, cluster3_id],
            );
            clusters.push(idx2);

            // println!(
            //     "cluster id: {} cut: {:?}",
            //     idx1,
            //     ctree.get(&idx1).cut
            // );
            // println!(
            //     "cluster id: {} cut: {:?}",
            //     idx2,
            //     ctree.get(&idx2).cut
            // );

            let removals = vec![cluster0_id, cluster1_id, cluster2_id, cluster3_id];
            let mut remove_idxs: Vec<usize> = (0..clusters.len())
                .filter(|idx| removals.contains(&clusters[*idx]))
                .collect();
            remove_idxs.sort();
            for r in remove_idxs.iter().rev() {
                clusters.remove(*r as usize);
            }

            // println!("cluster0_index: {}", cluster0_idx);
            // println!("cluster1_index: {}", cluster1_idx);
            // println!("cluster2_index: {}", cluster2_idx);
            // println!("cluster3_index: {}", cluster3_idx);
            let mut remove_idxs = [cluster0_idx, cluster1_idx, cluster2_idx, cluster3_idx];
            remove_idxs.sort();
            let mut old_shared = Vec::with_capacity(4);
            for r in remove_idxs.iter().rev() {
                old_shared.push(shared_edges.remove(*r as usize));
            }

            // println!("old_shared: {:?}", old_shared);

            // fix the overlapping edges on the other clusters in the lists
            let (mut connections1, mut connections2) = recalculate_shared_edges(
                &ctree,
                [cluster0_id, cluster1_id, cluster2_id, cluster3_id],
                [idx1, idx2],
                // &clusters,
                &old_shared
                    .iter()
                    .flat_map(|value| value.connections.clone())
                    .collect(),
                &mut shared_edges,
            );

            let mut lod_levels: Vec<u32> = old_shared.iter().map(|e| e.lod).collect();
            lod_levels.sort();
            let lod = *lod_levels.last().unwrap() + 1;
            connections1.sort_by(|a, b| b.1.cmp(&a.1));
            shared_edges.push(SharedEdges {
                id: idx1,
                lod,
                connections: connections1,
            });
            connections2.sort_by(|a, b| b.1.cmp(&a.1));
            shared_edges.push(SharedEdges {
                id: idx2,
                lod,
                connections: connections2,
            });

            if TESTING {
                let mut write_file = File::create("./logs/log1").unwrap();
                // let mut writer = BufWriter::new(&write_file);

                let t_shared_edges = {
                    let mut shared_edges: HashMap<u32, Vec<(u32, i32)>> = HashMap::new();
                    for id in 0..ctree.len() as u32 {
                        if ctree.has_parent(&id) {
                            continue;
                        }
                        let mut connections = Vec::with_capacity(10);
                        for j in 0..ctree.len() as u32 {
                            if id == j {
                                continue;
                            }

                            if ctree.has_parent(&j) {
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
                    }

                    let mut shared_edges: Vec<SharedEdges> = shared_edges
                        .into_iter()
                        .map(|v| {
                            // let mut connections = v.1;
                            // connections.sort_by(|a, b| b.1.cmp(&a.1));
                            SharedEdges {
                                id: v.0,
                                // TODO poll ctree to determine lod level
                                lod: ctree.get_lod(&v.0),
                                connections: v.1,
                            }
                        })
                        .collect();

                    shared_edges.sort_by(|element1, element2| {
                        let cmp = element2.lod.cmp(&element1.lod);

                        // ensure connections is sorted
                        if cmp == Ordering::Equal {
                            let cmp2 = element1
                                .connections
                                .get(0)
                                .cmp(&element2.connections.get(0));

                            if cmp2 == Ordering::Equal {
                                element1.id.cmp(&element2.id)
                            } else {
                                cmp2
                            }
                        } else {
                            cmp
                        }
                    });

                    shared_edges
                };

                // normally this happens at the top of the loop, needs to be sorted to compare though so do it here
                shared_edges.sort_by(|element1, element2| {
                    let cmp = element2.lod.cmp(&element1.lod);

                    // ensure connections is sorted
                    if cmp == Ordering::Equal {
                        let cmp2 = element1
                            .connections
                            .get(0)
                            .cmp(&element2.connections.get(0));

                        if cmp2 == Ordering::Equal {
                            element1.id.cmp(&element2.id)
                        } else {
                            cmp2
                        }
                    } else {
                        cmp
                    }
                });

                let shared_edges_vec;

                // compare t_incident_mesh
                {
                    shared_edges_vec =
                        compare_vecs_w_order_customeq(&shared_edges, &t_shared_edges, &|a, b| {
                            if a.id == b.id
                                && a.lod == b.lod
                                && compare_vecs_w_order(&a.connections, &b.connections)
                            {
                                true
                            } else {
                                false
                            }
                        });

                    if !shared_edges_vec {
                        write_file
                            .write_all(format!("shared_edges_t: \n").as_bytes())
                            .unwrap();
                        for se in &t_shared_edges {
                            write_file
                                .write_all(
                                    format!(
                                        "id: {} lod: {}, connections: {:?} \n",
                                        se.id, se.lod, se.connections
                                    )
                                    .as_bytes(),
                                )
                                .unwrap();
                            // write_file.flush().unwrap();
                        }
                        write_file
                            .write_all(format!("\n\n\n\n\n").as_bytes())
                            .unwrap();

                        // write_file.flush().unwrap();

                        write_file
                            .write_all(format!("shared_edges_o: \n").as_bytes())
                            .unwrap();
                        for se in &shared_edges {
                            write_file
                                .write_all(
                                    format!(
                                        "id: {} lod: {}, connections: {:?} \n",
                                        se.id, se.lod, se.connections
                                    )
                                    .as_bytes(),
                                )
                                .unwrap();

                            // write_file.flush().unwrap();
                        }
                        write_file
                            .write_all(format!("\n\n\n\n\n").as_bytes())
                            .unwrap();

                        write_file.flush().unwrap();
                    }
                };

                println!("Test Run");
                println!("shared_edges_vec: {}", shared_edges_vec);
                println!("Test Run Complete \n\n\n");

                if !(shared_edges_vec) {
                    panic!("Test Run Failed");
                }
            }
        }
        // simplify the large clusters triangles
        // split the cluster into two clusters of TRIS_IN_CLUSTER size
        // add the clusters back to the list of clusters to be combined
        // end when there is only one cluster leftover
    }
    // !SECTION - Combine the clusters into a graph

    let mut ctree = ctree.to_official(&mesh.positions);

    // for cluster in &ctree.clusters {
    //     println!("cluster positions len: {}", cluster.mesh.positions.len());
    // }

    // write_clusters(ctree.clusters());

    // !SECTION - Write ctree to file
    {
        // write graph to file (nodes with children)

        // write clusters to file

        ctree.write_to_file("test_output.cm");
    };
    // !SECTION - Write ctree to file

    println!("og len: {}", ctree.len());

    println!("finished");

    ctree
}

// fn add_tri(
//     tri: &Triangle,
//     tris: &mut HashSet<Triangle>,
//     edges: &mut HashMap<Edge, [u32; 2]>,
//     overall_cluster_cut: &mut HashSet<Edge>,
//     cur_cut: &mut HashSet<Edge>,
// ) {
//     tris.insert(*tri);
//
//     let edge0 = new_edge(tri[0], tri[1]);
//     let edge1 = new_edge(tri[0], tri[2]);
//     let edge2 = new_edge(tri[1], tri[2]);
//
//     let mut add0 = false;
//     let mut add1 = false;
//     let mut add2 = false;
//
//     if let Some(value) = edges.get_mut(&edge0) {
//         if value[0] == u32::MAX {
//             value[0] = tri[2];
//         } else if value[1] == u32::MAX {
//             value[1] = tri[2];
//         }
//     } else {
//         add0 = true;
//     }
//
//     if let Some(value) = edges.get_mut(&edge1) {
//         if value[0] == u32::MAX {
//             value[0] = tri[1];
//         } else if value[1] == u32::MAX {
//             value[1] = tri[1];
//         }
//     } else {
//         add1 = true;
//     }
//
//     if let Some(value) = edges.get_mut(&edge2) {
//         if value[0] == u32::MAX {
//             value[0] = tri[0];
//         } else if value[1] == u32::MAX {
//             value[1] = tri[0];
//         }
//     } else {
//         add2 = true;
//     }
//
//     if add0 {
//         if overall_cluster_cut.contains(&edge0) {
//             // cur_cut.insert(edge0);
//             overall_cluster_cut.remove(&edge0);
//         } else {
//             // cur_cut.remove(&edge0);
//             overall_cluster_cut.insert(edge0);
//         }
//         edges.insert(edge0, new_edge(tri[2], u32::MAX));
//     }
//     // else {
//     //     cur_cut.insert(edge0);
//     // }
//
//     if add1 {
//         if !overall_cluster_cut.contains(&edge1) {
//             // cur_cut.insert(edge1);
//             overall_cluster_cut.remove(&edge1);
//         } else {
//             // cur_cut.insert(edge1);
//             overall_cluster_cut.insert(edge1);
//         }
//         edges.insert(edge1, new_edge(tri[1], u32::MAX));
//     }
//     // else {
//     //     cur_cut.insert(edge1);
//     // }
//
//     if add2 {
//         if !overall_cluster_cut.contains(&edge2) {
//             // cur_cut.insert(edge2);
//             overall_cluster_cut.remove(&edge2);
//         } else {
//             // cur_cut.insert(edge2);
//             overall_cluster_cut.insert(edge2);
//         }
//         edges.insert(edge2, new_edge(tri[0], u32::MAX));
//     }
//     // else {
//     //     cur_cut.insert(edge2);
//     // }
//
//     if !cur_cut.contains(&edge0) {
//         cur_cut.insert(edge0);
//     } else {
//         cur_cut.remove(&edge0);
//     }
//
//     if !cur_cut.contains(&edge1) {
//         cur_cut.insert(edge1);
//     } else {
//         cur_cut.remove(&edge1);
//     }
//
//     if !cur_cut.contains(&edge2) {
//         cur_cut.insert(edge2);
//     } else {
//         cur_cut.remove(&edge2);
//     }
// }

fn recalculate_shared_edges(
    ctree: &TempCTree,
    combined: [u32; 4],
    new: [u32; 2],
    // clusters: &Vec<u32>,
    connections: &Vec<(u32, i32)>,
    shared_edges: &mut Vec<SharedEdges>,
) -> (Vec<(u32, i32)>, Vec<(u32, i32)>) {
    let connections: HashSet<u32> = connections.iter().map(|x| x.0).collect();
    let mut connections1 = Vec::new();
    let mut connections2 = Vec::new();
    // println!("connections to test: {:?}", connections);

    // test first
    {
        let new0 = ctree.get(&new[0]);
        for connection in &connections {
            if combined.contains(connection) {
                continue;
            }

            let connected = ctree.get(connection);
            let count = new0.cut.intersection(&connected.cut).count();
            if count > 0 {
                connections1.push((*connection, count as i32));
            }
        }
    };
    // test second
    {
        let new1 = ctree.get(&new[1]);
        for connection in &connections {
            if combined.contains(connection) {
                continue;
            }

            let connected = ctree.get(connection);
            let count = new1.cut.intersection(&connected.cut).count();
            if count > 0 {
                connections2.push((*connection, count as i32));
            }
        }
    };

    {
        let new0 = ctree.get(&new[0]);
        let new1 = ctree.get(&new[1]);
        let count = new0.cut.intersection(&new1.cut).count();
        if count > 0 {
            connections1.push((new[1], count as i32));
            connections2.push((new[0], count as i32));
        }
    };

    for sedge_idx in 0..shared_edges.len() {
        let mut modified = false;
        {
            let mut removals = Vec::new();
            let connections = &mut shared_edges[sedge_idx].connections;
            for idx in 0..connections.len() {
                if combined.contains(&connections[idx].0) {
                    removals.push(idx);
                    modified = true;
                }
            }
            for r in removals.iter().rev() {
                connections.remove(*r);
            }
        }

        for c in &connections1 {
            if c.0 == shared_edges[sedge_idx].id {
                shared_edges[sedge_idx].connections.push((new[0], c.1));
                modified = true;
                break;
            }
        }

        for c in &connections2 {
            if c.0 == shared_edges[sedge_idx].id {
                shared_edges[sedge_idx].connections.push((new[1], c.1));
                modified = true;
                break;
            }
        }

        if modified {
            shared_edges[sedge_idx].connections.sort_by(|a, b| {
                let cmp = b.1.cmp(&a.1);
                if cmp == Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    cmp
                }
            });
        }
    }

    // println!("cluster id: {} connections: {:?}", new[0], connections1);
    // println!("cluster id: {} connections: {:?}", new[1], connections2);

    connections1.sort_by(|a, b| {
        let cmp = b.1.cmp(&a.1);
        if cmp == Ordering::Equal {
            a.0.cmp(&b.0)
        } else {
            cmp
        }
    });
    connections2.sort_by(|a, b| {
        let cmp = b.1.cmp(&a.1);
        if cmp == Ordering::Equal {
            a.0.cmp(&b.0)
        } else {
            cmp
        }
    });

    (connections1, connections2)
}

fn write_clusters(clusters: &Vec<Cluster>) {
    // let mut rng = rand::thread_rng();

    let mut f = File::create("clusters.obj").unwrap();
    f.write_all(b"mtllib bunny.mtl\no bun_zipper\n").unwrap();

    let mut offsets = Vec::new();
    let mut offset = 0;
    for cluster in clusters {
        // f.write_all(format!("o Cluster{}\n", idx + 1).as_bytes())
        //     .unwrap();
        // let color: [f32; 3] = [rng.gen(), rng.gen(), rng.gen()];
        offsets.push(offset);
        for pos in (0..cluster.mesh.positions.len()).step_by(3) {
            f.write_all(
                format!(
                    "v {} {} {} \n", // {} {} {}
                    cluster.mesh.positions[pos + 0],
                    cluster.mesh.positions[pos + 1],
                    cluster.mesh.positions[pos + 2],
                    // color[0],
                    // color[1],
                    // color[2]
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all("\n".as_bytes()).unwrap();
        offset += cluster.mesh.positions.len() / 3;
    }

    f.write_all(b"usemtl None\ns off\n").unwrap();

    for (idx, cluster) in clusters.iter().enumerate() {
        f.write_all(format!("o Cluster{}\n", idx + 1).as_bytes())
            .unwrap();
        let offset = offsets[idx];
        for idx in (0..cluster.mesh.indices.len()).step_by(3) {
            f.write_all(
                format!(
                    "f {} {} {}\n",
                    cluster.mesh.indices[idx + 0] + 1 + offset as u32,
                    cluster.mesh.indices[idx + 1] + 1 + offset as u32,
                    cluster.mesh.indices[idx + 2] + 1 + offset as u32
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all("\n".as_bytes()).unwrap();
    }

    f.flush().unwrap();
}
