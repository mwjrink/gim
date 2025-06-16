use crate::debug::dump_part;
use crate::*;
use foldhash::{HashMap, HashMapExt};
use interop::TRIS_IN_CLUSTER;
use std::usize;
use ultraviolet::Vec3;

pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    // dump_raw(mesh, "RAW".to_string());

    let mut algo_mesh = AlgoMesh {
        triangles: {
            mesh.indices
                .chunks(3)
                // .filter(|chunk| {
                //     chunk[0] != chunk[1] && chunk[0] != chunk[2] && chunk[1] != chunk[2]
                // })
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
                                // these are degenerate triangles, two verts are the same
                                if adjacent.connections[connection_idx] != usize::MAX
                                    && tri_idx != *adjacent_idx
                                {
                                    panic!();
                                } else if adjacent.connections[connection_idx] != usize::MAX
                                    && tri_idx == *adjacent_idx
                                {
                                    set_connection = true;
                                    continue;
                                }

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

        // let mesh = metis::Mesh::new(1, 1, &eptr, &eind).unwrap();
        // let mut partmesh = vec![0i64; algo_mesh.triangles.len()];
        // mesh.part_dual(epart, npart);
        // mesh.part_nodal(epart, npart);
        let mut adj = Vec::with_capacity(mesh.indices.len());
        let mut xadj = Vec::with_capacity(mesh.indices.len() / 3);

        xadj.push(0);
        let mut num_connections = 0;
        for tri in &algo_mesh.triangles {
            for connection in &tri.connections {
                if *connection != usize::MAX {
                    adj.push(*connection as i32);
                    num_connections += 1;
                }
            }
            xadj.push(num_connections);
        }

        // the -1 keeps this in the range of 126 - 128
        let deviation = 1;
        let desired = (algo_mesh.triangles.len() / (TRIS_IN_CLUSTER - deviation as usize)) as i32;
        let parts_num =
            unsafe { (desired as f32 * (1.0 + 0.01 * deviation as f32)).to_int_unchecked() };
        // let desired = TRIS_IN_CLUSTER as i32;
        let graph = metis::Graph::new(1, parts_num, &xadj, &adj)
            .unwrap()
            // .set_option(metis::option::Contig(true))
            .set_option(metis::option::ObjType::Vol)
            .set_option(metis::option::IpType::Grow)
            .set_option(metis::option::UFactor(deviation))
            // .set_option(metis::option::MinConn(true))
            // .set_option(metis::option::ObjType::Vol)
            .set_option(metis::option::PType::Kway);

        let mut part = vec![0; algo_mesh.triangles.len()];
        let cut = graph.part_kway(&mut part).unwrap();
        // let cut = graph.part_recursive(&mut part).unwrap();
        println!("Desired: {}", desired);
        println!("Cut: {}", cut);
        let num_parts = *part.iter().max().unwrap() as usize + 1;
        println!("Part: {:?}", num_parts);
        let mut clusters = vec![Vec::with_capacity(TRIS_IN_CLUSTER); num_parts];

        for (tri_idx, cluster_idx) in part.iter().enumerate() {
            clusters[*cluster_idx as usize].push(tri_idx);
            algo_mesh.triangles[tri_idx].owned_by = *cluster_idx as usize;
        }

        let mut num_degen = 0;
        let mut num_exact = 0;
        let mut num_fat = 0;
        for (idx, cluster) in clusters.iter().enumerate() {
            if cluster.len() < TRIS_IN_CLUSTER {
                // println!(
                //     "{} is a degenerate cluster with {} tris",
                //     idx,
                //     cluster.len()
                // )
                num_degen += 1;
            } else if cluster.len() == TRIS_IN_CLUSTER {
                // println!(
                //     "{} is NOT a degenerate cluster with {} tris",
                //     idx,
                //     cluster.len()
                // )
                num_exact += 1;
            } else {
                // println!(
                //     "{} is a FAT GLUTONOUS cluster with {} tris",
                //     idx,
                //     cluster.len()
                // )
                num_fat += 1;
                panic!("We have a fat cluster... ouch.");
            }
        }
        println!(
            "\n Degen: {}\n Fat: {}\n Exact: {}",
            num_degen, num_fat, num_exact
        );

        println!("Done subdividing.");
        println!("Dumping.");
        dump_part(
            &mesh.vertices,
            &algo_mesh.triangles,
            &clusters,
            "METISKWay".to_string(),
        );
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
