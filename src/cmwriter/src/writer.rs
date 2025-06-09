use std::{f32, u64};

use interop::TESTING;
use interop::{Vertex, TRIS_IN_CLUSTER};
use ultraviolet::Vec3;

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone)]
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

struct Triangle {
    idx: u64,
    taken_by: u64,
    anchor: Vec3,
    // edge1, edge2, implicit_edge
    connections: [u32; 3],
}

struct AlgoMesh {
    pub triangles: Vec<Triangle>,
}

struct AlgoMeshSubset {
    pub start: usize,
    pub len: usize,
}

// TODO add a tri trading step where clusters trade triangles
pub fn write(mesh: &mut Mesh) -> CTree {
    let mut nodes = Vec::new();
    let mut clusters = Vec::new();

    let mut maximum = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut minimum = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut subsets = Vec::new();
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

                    for (idx, val) in vert0.position.as_array().iter().enumerate() {
                        if val < &minimum.as_array()[idx] {
                            minimum.as_mut_array()[idx] = *val;
                        }

                        if val > &maximum.as_array()[idx] {
                            maximum.as_mut_array()[idx] = *val;
                        }
                    }

                    Triangle {
                        idx: idx as u64,
                        taken_by: 0,
                        anchor: geo_mean,
                        connections: [u32::MAX, u32::MAX, u32::MAX],
                    }
                })
                .collect()
        },
    };
    println!("Done calcing the geo means.");

    // Fill connections in algo_mesh triangles
    {
        // -
    }

    // start splitting
    {
        println!("Splitting...");
        // -
        let seed_point_0 = minimum;
        let mut tris0 = Vec::with_capacity(algo_mesh.triangles.len() / 2 + TRIS_IN_CLUSTER);
        subsets.push(AlgoMeshSubset {
            start: 0,
            len: algo_mesh.triangles.len(),
        });

        let seed_point_1 = maximum;
        let mut tris1 = Vec::with_capacity(algo_mesh.triangles.len() / 2 + TRIS_IN_CLUSTER);
        subsets.push(AlgoMeshSubset {
            start: algo_mesh.triangles.len() - 1,
            len: 0,
        });

        for (idx, triangle) in algo_mesh.triangles.iter_mut().enumerate() {
            let dist0 = (seed_point_0 - triangle.anchor).mag_sq();
            let dist1 = (seed_point_1 - triangle.anchor).mag_sq();
            if dist0 > dist1 {
                triangle.taken_by = 0;
                tris0.push(idx);
            } else {
                triangle.taken_by = 1;
                tris1.push(idx);
            }
        }

        let imbalance = tris0.len().abs_diff(tris1.len()) / 2;
        println!(
            "Initial split has: {} vs {}, imbalance: {}",
            tris0.len(),
            tris1.len(),
            imbalance
        );

        let total = tris0.len() + tris1.len();

        // balance
        if imbalance > TRIS_IN_CLUSTER {
            let (
                mut add_to_list,
                add_anchor,
                add_idx,
                mut rem_from_list,
                rem_anchor,
                rem_idx,
                amount,
            ) = if tris0.len() > tris1.len() {
                let amount = imbalance - ((tris1.len() + imbalance) % TRIS_IN_CLUSTER);
                (tris1, seed_point_1, 1u64, tris0, seed_point_0, 0u64, amount)
            } else {
                let amount = imbalance - ((tris0.len() + imbalance) % TRIS_IN_CLUSTER);
                (tris0, seed_point_0, 0u64, tris1, seed_point_1, 1u64, amount)
            };

            println!("Balancing {}...", amount);
            // let mut closest = Vec::new();
            // closest.resize(amount, (0usize, 0usize, f32::MAX));
            // TODO this might be faster if I cached the distances?
            //     - takes like a second for lucy (28 mill tris)
            rem_from_list.sort_by(|a, b| {
                let a_triangle = &mut algo_mesh.triangles[*a];
                let a_dist = (add_anchor - a_triangle.anchor).mag_sq();

                let b_triangle = &mut algo_mesh.triangles[*b];
                let b_dist = (add_anchor - b_triangle.anchor).mag_sq();

                b_dist.total_cmp(&a_dist)
            });

            // TODO only take the tris that are edge or hole tris here
            for rem_idx in rem_from_list.drain((rem_from_list.len() - amount)..rem_from_list.len())
            {
                add_to_list.push(rem_idx);
            }

            assert_eq!(total, rem_from_list.len() + add_to_list.len());
            let imbalance = add_to_list.len().abs_diff(rem_from_list.len());
            println!(
                "After balancing: {} vs {}, imbalance: {}",
                rem_from_list.len(),
                add_to_list.len(),
                imbalance
            );

            // TODO ensure the cut is contiguous here
            rem_from_list.sort_by(|a, b| a.cmp(&b));
            add_to_list.sort_by(|a, b| b.cmp(&a));
            let mut swapped;
            loop {
                swapped = false;
                for (asc, desc) in rem_from_list.iter_mut().zip(add_to_list.iter_mut()) {
                    if asc > desc {
                        algo_mesh.triangles.swap(*desc, *asc);
                        std::mem::swap(asc, desc);
                        swapped = true;
                    }
                }

                rem_from_list.sort_by(|a, b| a.cmp(&b));
                add_to_list.sort_by(|a, b| b.cmp(&a));

                if !swapped {
                    break;
                }
            }

            println!("rem: {:?}", &rem_from_list[0..10]);
            println!("add: {:?}", &add_to_list[0..10]);

            rem_from_list.sort_by(|a, b| a.cmp(&b));
            add_to_list.sort_by(|a, b| b.cmp(&a));
            println!(
                "Should eq: {} {}",
                *rem_from_list.last().unwrap(),
                rem_from_list.len()
            );
            assert!(*rem_from_list.last().unwrap() == rem_from_list.len());

            println!(
                "Should eq: {} {}",
                *add_to_list.first().unwrap(),
                rem_from_list.len()
            );
            assert!(*add_to_list.first().unwrap() == rem_from_list.len());

            println!(
                "Should eq: {} {}",
                *add_to_list.last().unwrap(),
                algo_mesh.triangles.len()
            );
            assert!(*add_to_list.last().unwrap() == algo_mesh.triangles.len());
            println!("Done swapping.");
        }

        // swap triangle positions in the vec so it can be split evenly
        // algo_mesh.triangles.swap(a, b);
        // algo_mesh.triangles.swap_unchecked(a, b);
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
