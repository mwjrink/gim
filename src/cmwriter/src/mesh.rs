use crate::utils::*;
use crate::vertex::Vertex;
use core::panic;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};

const TESTING: bool = false;

pub type Triangle = [u32; 3];
pub type Edge = [u32; 2];
pub type Point = [f32; 3];

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone)]
pub struct TempMesh {
    pub indices: Vec<u32>,
}

pub fn simplify(
    mesh: &TempMesh,
    positions: &mut Vec<f32>,
    mesh_edge: &HashSet<Edge>,
    target_tris: u32,
) -> TempMesh {
    let generate_necessary = |mesh: &TempMesh, positions: &Vec<f32>| {
        let incident_mesh = TempMesh {
            // positions: positions.clone(),
            indices: mesh.indices.clone(),
        };

        let (edges, tris, vertices, (vertex_map, tri_list)) =
            generate_data_structures(&incident_mesh.indices);

        // Every cluster has their own list of positions

        let mut edge_verts = HashSet::<u32>::new();
        for edge in &edges {
            if edge.1[1] == u32::MAX || edge.1[0] == u32::MAX {
                if !mesh_edge.contains(edge.0) {
                    edge_verts.insert(edge.0[0]);
                    edge_verts.insert(edge.0[1]);
                }
            }
        }

        // println!("edge_verts: {:?}", edge_verts);

        let mut collapsible = Vec::<(f32, Edge)>::with_capacity(edges.len());
        for edge in &edges {
            if !edge_verts.contains(&edge.0[0]) && !edge_verts.contains(&edge.0[1]) {
                collapsible.push((sqr_dist(&positions, edge.0[0], edge.0[1]), *edge.0));
            }
            // else {
            //     println!("rejected: {:?}", *edge.0);
            // }
        }

        // TODO
        let mut tri_dict = HashMap::new();
        for tri in &tris {
            {
                let value = tri_dict.insert(tri[0], vec![new_edge(tri[1], tri[2])]);
                if let Some(mut old) = value {
                    if let Some(list) = tri_dict.get_mut(&tri[0]) {
                        list.append(&mut old);
                    }
                }
            };

            {
                let value = tri_dict.insert(tri[1], vec![new_edge(tri[0], tri[2])]);
                if let Some(mut old) = value {
                    if let Some(list) = tri_dict.get_mut(&tri[1]) {
                        list.append(&mut old);
                    }
                }
            };

            {
                let value = tri_dict.insert(tri[2], vec![new_edge(tri[0], tri[1])]);
                if let Some(mut old) = value {
                    if let Some(list) = tri_dict.get_mut(&tri[2]) {
                        list.append(&mut old);
                    }
                }
            };
        }

        collapsible.sort_by(|one, two| one.0.partial_cmp(&two.0).unwrap());

        (
            incident_mesh,
            vertices,
            tris,
            edges,
            collapsible,
            tri_dict,
            vertex_map,
            tri_list,
        )
    };

    // let mut flipped_count = 0;

    let (
        mut incident_mesh, //
        mut vertices,
        mut tris,
        mut edges,
        mut collapsible,
        mut tri_dict,
        vertex_map,
        tri_list,
    ) = generate_necessary(mesh, positions);

    while incident_mesh.indices.len() > (target_tris * 3) as usize {
        let is_collapsible = |edge: &Edge| {
            let midpoint = mid_point_2(positions, edge[0], edge[1]);
            // exactly 2 neigbouring vertices for each element in edge
            let mut a = vertices.get(&edge[0]).unwrap().clone();
            a.sort();
            let mut b = vertices.get(&edge[1]).unwrap().clone();
            b.sort();
            let intsxn = sorted_vec_intersection_count(&a, &b);

            if intsxn > 2 {
                // println!("shared verts condition: {}", intsxn);
                return false;
            }

            // detect flipped triangles

            // for every triangle with one of these vertices in it, calculate the cross product of the trangle, then again with the new vert, if the dot product between those two is negative, the triangle was flipped
            for tri in tri_dict.get(&edge[0]).unwrap() {
                let edge1 = [
                    positions[tri[1] as usize * 3 + 0] - positions[tri[0] as usize * 3 + 0],
                    positions[tri[1] as usize * 3 + 1] - positions[tri[0] as usize * 3 + 1],
                    positions[tri[1] as usize * 3 + 2] - positions[tri[0] as usize * 3 + 2],
                ];

                let initial = {
                    let edge2 = [
                        positions[edge[0] as usize * 3 + 0] - positions[tri[0] as usize * 3 + 0],
                        positions[edge[0] as usize * 3 + 1] - positions[tri[0] as usize * 3 + 1],
                        positions[edge[0] as usize * 3 + 2] - positions[tri[0] as usize * 3 + 2],
                    ];

                    [
                        (edge1[1] * edge2[2] - edge1[2] * edge2[1]),
                        (edge1[2] * edge2[0] - edge1[0] * edge2[2]),
                        (edge1[0] * edge2[1] - edge1[1] * edge2[0]),
                    ]
                };

                let second = {
                    let edge2 = [
                        midpoint[0] - positions[tri[0] as usize * 3 + 0],
                        midpoint[1] - positions[tri[0] as usize * 3 + 1],
                        midpoint[2] - positions[tri[0] as usize * 3 + 2],
                    ];

                    [
                        (edge1[1] * edge2[2] - edge1[2] * edge2[1]),
                        (edge1[2] * edge2[0] - edge1[0] * edge2[2]),
                        (edge1[0] * edge2[1] - edge1[1] * edge2[0]),
                    ]
                };

                let dot_product =
                    initial[0] * second[0] + initial[1] * second[1] + initial[2] * second[2];
                if dot_product < 0.0 {
                    return false;
                }
            }

            for tri in tri_dict.get(&edge[1]).unwrap() {
                let edge1 = [
                    positions[tri[1] as usize * 3 + 0] - positions[tri[0] as usize * 3 + 0],
                    positions[tri[1] as usize * 3 + 1] - positions[tri[0] as usize * 3 + 1],
                    positions[tri[1] as usize * 3 + 2] - positions[tri[0] as usize * 3 + 2],
                ];

                let initial = {
                    let edge2 = [
                        positions[edge[1] as usize * 3 + 0] - positions[tri[0] as usize * 3 + 0],
                        positions[edge[1] as usize * 3 + 1] - positions[tri[0] as usize * 3 + 1],
                        positions[edge[1] as usize * 3 + 2] - positions[tri[0] as usize * 3 + 2],
                    ];

                    [
                        (edge1[1] * edge2[2] - edge1[2] * edge2[1]),
                        (edge1[2] * edge2[0] - edge1[0] * edge2[2]),
                        (edge1[0] * edge2[1] - edge1[1] * edge2[0]),
                    ]
                };

                let second = {
                    let edge2 = [
                        midpoint[0] - positions[tri[0] as usize * 3 + 0],
                        midpoint[1] - positions[tri[0] as usize * 3 + 1],
                        midpoint[2] - positions[tri[0] as usize * 3 + 2],
                    ];

                    [
                        (edge1[1] * edge2[2] - edge1[2] * edge2[1]),
                        (edge1[2] * edge2[0] - edge1[0] * edge2[2]),
                        (edge1[0] * edge2[1] - edge1[1] * edge2[0]),
                    ]
                };

                let dot_product =
                    initial[0] * second[0] + initial[1] * second[1] + initial[2] * second[2];
                if dot_product < 0.0 {
                    return false;
                }
            }

            true
        };

        let collapse = |edge: &Edge, incident_mesh: &mut TempMesh, positions: &mut Vec<f32>| {
            let midpoint = mid_point_2(positions, edge[0], edge[1]);
            // -TEMP simplify run at the end will remove all unused
            // remove the original 2 indices, the vec shortens
            //
            // removing is possible but super problematic, need to
            // decrement the indices above the collapsed for every
            // index in every data structure, safer for now to just
            // do it at the end as this is likely not an impact on
            // performance
            //
            // // incident_mesh.positions.remove((edge[0] + 0) as usize);
            // // incident_mesh.positions.remove((edge[0] + 0) as usize);
            // // incident_mesh.positions.remove((edge[0] + 0) as usize);
            // //
            // // incident_mesh.positions.remove((edge[1] + 0) as usize);
            // // incident_mesh.positions.remove((edge[1] + 0) as usize);
            // // incident_mesh.positions.remove((edge[1] + 0) as usize);

            let new_idx = positions.len() / 3;
            positions.extend_from_slice(&midpoint);
            // positions.push(midpoint[0]);
            // positions.push(midpoint[1]);
            // positions.push(midpoint[2]);

            // move all indices pointing to an index that got moved
            for idx in &mut incident_mesh.indices {
                if *idx == edge[0] || *idx == edge[1] {
                    *idx = new_idx as u32;
                }

                // see "// -TEMP simplify run at..."
                // //                 if *idx > edge[0] {
                // //                     *idx -= 1;
                // //                 }
                // //
                // //                 if *idx > edge[1] {
                // //                     *idx -= 1;
                // //                 }
            }

            let mut removals = Vec::with_capacity(2);
            for idx in (0..incident_mesh.indices.len()).step_by(3) {
                // removing degenerate triangles, ie vert = vert
                if incident_mesh.indices[idx] == incident_mesh.indices[idx + 1]
                    || incident_mesh.indices[idx] == incident_mesh.indices[idx + 2]
                    || incident_mesh.indices[idx + 1] == incident_mesh.indices[idx + 2]
                {
                    removals.push(idx);
                }
            }
            // removing a triangle, remove the index of the first then second then third vertices in the triangle
            // removals.sort_by(|a, b| { b.cmp(a) });
            for r in removals.into_iter().rev() {
                incident_mesh.indices.remove(r);
                incident_mesh.indices.remove(r);
                incident_mesh.indices.remove(r);
            }

            (midpoint, new_idx as u32)
        };

        if collapsible.is_empty() {
            // println!("positions: {}", positions.len());
            println!("indices: {}", incident_mesh.indices.len());

            // incident_mesh.write_mesh_to_file("debug_cluster_edge_collapsing", positions);

            panic!("This mesh cannot be simplified (no collapsibles)");
        }

        let mut idx = 0;
        while !is_collapsible(&collapsible[idx].1) {
            idx += 1;

            if idx == collapsible.len() {
                // println!("positions: {}", positions.len());
                println!("indices: {}", incident_mesh.indices.len());
                println!(
                    "This mesh cannot be simplified further (no edge passed collapsible test)"
                );

                // incident_mesh.write_mesh_to_file("debug_cluster_edge_collapsing", positions);

                // panic!("This mesh cannot be simplified");
                return incident_mesh;
                // panic!("not further collapsible");
            }
        }
        let collapsed = collapsible[idx].1;

        // println!("collapsed: {:?}", collapsed);
        let new_point = collapse(&collapsed, &mut incident_mesh, positions);

        // sanity check
        let removed_collapsed = collapsible.remove(idx);
        assert!(collapsed[0] == removed_collapsed.1[0] && collapsed[1] == removed_collapsed.1[1]);

        // need to combine the edges that end up being the same
        edges.remove(&collapsed);
        {
            let mut removals = Vec::new();
            let mut additions = Vec::new();
            for kv in &mut edges {
                let mut replaced = 0;
                let mut key_changed = false;

                let mut new_kv = (kv.0.clone(), kv.1.clone());
                if new_kv.0[0] == collapsed[0] || new_kv.0[0] == collapsed[1] {
                    new_kv.0[0] = new_point.1;
                    key_changed = true;
                    replaced += 1;
                }

                if new_kv.0[1] == collapsed[0] || new_kv.0[1] == collapsed[1] {
                    new_kv.0[1] = new_point.1;
                    key_changed = true;
                    replaced += 1;
                }

                if kv.1[0] == collapsed[0] || kv.1[0] == collapsed[1] {
                    if key_changed {
                        new_kv.1[0] = u32::MAX;
                    } else {
                        kv.1[0] = new_point.1;
                    }
                    replaced += 1;
                }

                if kv.1[1] == collapsed[0] || kv.1[1] == collapsed[1] {
                    if key_changed {
                        new_kv.1[1] = u32::MAX;
                    } else {
                        kv.1[1] = new_point.1;
                    }
                    replaced += 1;
                }

                if replaced >= 2 || key_changed {
                    removals.push(kv.0.clone());
                } else if replaced > 0 {
                    *kv.1 = new_edge(kv.1[0], kv.1[1]);
                }

                // if new_kv.1[0] == u32::MAX && new_kv.1[1] == u32::MAX {
                //     continue;
                // }

                if key_changed {
                    additions.push(new_kv);
                }
            }

            for r in removals {
                edges.remove(&r);
            }

            for a in additions {
                let key = new_edge(a.0[0], a.0[1]);
                let val = new_edge(a.1[0], a.1[1]);
                let old = edges.insert(key, val);
                if let Some(old) = old {
                    // println!("for key: {:?}, old: {:?}, value: {:?}", key, old, value);
                    let new = new_edge(
                        if old[0] == u32::MAX { old[1] } else { old[0] },
                        if val[0] == u32::MAX { val[1] } else { val[0] },
                    );
                    // assert!(if old[0] == u32::MAX { old[1] } else { old[0] } != u32::MAX);
                    // assert!(if val[0] == u32::MAX { val[1] } else { val[0] } != u32::MAX);

                    if new[0] != u32::MAX || new[1] != u32::MAX {
                        edges.insert(key, new);
                    }
                }
            }
        }

        // tris.remove(r);
        {
            let mut removals = Vec::new();
            let mut additions = Vec::new();
            for tri in tris.iter() {
                let mut count = 0;
                let mut add: Triangle = [0, 0, 0];
                if tri[0] == collapsed[0] || tri[0] == collapsed[1] {
                    count += 1;
                    add = new_tri(new_point.1, tri[1], tri[2]);
                }
                if tri[1] == collapsed[0] || tri[1] == collapsed[1] {
                    count += 1;
                    add = new_tri(new_point.1, tri[0], tri[2]);
                }
                if tri[2] == collapsed[0] || tri[2] == collapsed[1] {
                    count += 1;
                    add = new_tri(new_point.1, tri[0], tri[1]);
                }

                if count > 1 {
                    removals.push(tri.clone());
                }

                if count == 1 {
                    removals.push(tri.clone());
                    additions.push(add);
                }
            }
            for r in removals {
                tris.remove(&r);
            }
            for add in additions {
                tris.insert(add);
            }
        }

        // vertices.remove();
        {
            let mut combined = Vec::<u32>::new();
            let a = vertices.get(&collapsed[0]).unwrap().clone();
            let b = vertices.get(&collapsed[1]).unwrap().clone();
            let mut adjacent = HashSet::with_capacity(a.len() + b.len());
            adjacent.extend(a);
            adjacent.extend(b);
            for adj in &adjacent {
                let mut idx = None;
                let mut already_in = false;
                for vert in vertices.get_mut(adj).unwrap().iter_mut().enumerate() {
                    if *vert.1 == collapsed[0] || *vert.1 == collapsed[1] {
                        *vert.1 = new_point.1;
                        if already_in {
                            idx = Some(vert.0);
                        } else {
                            already_in = true;
                        }
                    }
                }

                if let Some(value) = idx {
                    vertices.get_mut(adj).unwrap().swap_remove(value);
                }
            }
            combined.extend(adjacent);

            let mut idx = 0;
            while idx < combined.len() {
                if combined[idx] == collapsed[0] {
                    combined.remove(idx);
                } else if combined[idx] == collapsed[1] {
                    combined.remove(idx);
                } else {
                    idx += 1;
                }
            }

            vertices.remove(&collapsed[0]);
            vertices.remove(&collapsed[1]);

            vertices.insert(new_point.1, combined);
        }

        // tri_dict.remove();
        // (67, [[84, 161], [66, 74], [294, 294]
        // (55, [[294, 294],
        {
            let mut swaps = Vec::new();
            let mut removals = Vec::new();
            let mut combined = Vec::new();
            for edge in tri_dict.get(&collapsed[0]).unwrap() {
                if edge[0] == collapsed[1] || edge[1] == collapsed[1] {
                    removals.push(edge[0]);
                    removals.push(edge[1]);
                } else {
                    swaps.push(edge[0]);
                    swaps.push(edge[1]);
                    combined.push(*edge);
                }
            }

            for edge in tri_dict.get(&collapsed[1]).unwrap() {
                if edge[0] == collapsed[0] || edge[1] == collapsed[0] {
                    removals.push(edge[0]);
                    removals.push(edge[1]);
                } else {
                    swaps.push(edge[0]);
                    swaps.push(edge[1]);
                    combined.push(*edge);
                }
            }

            for swap in swaps {
                let mut idx = 0;
                let mut rs = Vec::new();
                for edge in tri_dict.get_mut(&swap).unwrap() {
                    if edge[0] == collapsed[0] || edge[0] == collapsed[1] {
                        edge[0] = new_point.1;
                    }

                    if edge[1] == collapsed[0] || edge[1] == collapsed[1] {
                        edge[1] = new_point.1;
                    }

                    if edge[0] == edge[1] {
                        rs.push(idx);
                    }

                    *edge = new_edge(edge[0], edge[1]);
                    idx += 1;
                }

                let edges = tri_dict.get_mut(&swap).unwrap();
                for r in rs {
                    edges.remove(r);
                }
            }

            for rem in removals {
                let mut removals = Vec::new();
                for edge in tri_dict.get(&rem).unwrap().iter().enumerate() {
                    if edge.1[0] == collapsed[1] || edge.1[1] == collapsed[1] {
                        removals.push(edge.0);
                    }
                }
                let m = tri_dict.get_mut(&rem).unwrap();
                for r in removals.into_iter().rev() {
                    m.remove(r);
                }
            }

            tri_dict.remove(&collapsed[0]);
            tri_dict.remove(&collapsed[1]);
            tri_dict.insert(new_point.1, combined);
        }

        // collapsible
        {
            // TODO I think I am missing the edges that go from the new point to the vertices shared by the collapsed values
            let mut contained = Vec::new();
            let mut removals = Vec::new();
            let mut idx: usize = 0;
            for edge in &mut collapsible {
                if edge.1[0] == collapsed[0] || edge.1[0] == collapsed[1] {
                    edge.1[0] = new_point.1;
                    if contained.contains(&edge.1[1]) {
                        removals.push(idx);
                    } else {
                        contained.push(edge.1[1]);
                        edge.0 = sqr_dist(positions, edge.1[0], edge.1[1]);
                        edge.1 = new_edge(edge.1[0], edge.1[1]);
                    }
                } else if edge.1[1] == collapsed[0] || edge.1[1] == collapsed[1] {
                    edge.1[1] = new_point.1;
                    if contained.contains(&edge.1[0]) {
                        removals.push(idx);
                    } else {
                        contained.push(edge.1[0]);
                        edge.0 = sqr_dist(positions, edge.1[0], edge.1[1]);
                        edge.1 = new_edge(edge.1[0], edge.1[1]);
                    }
                }

                idx += 1;
            }

            for r in removals.into_iter().rev() {
                // let removed =
                collapsible.remove(r);
                // println!("removed: {:?}", removed);
            }
            // println!("contained: {:?}", contained);

            collapsible.sort_by(|one, two| one.0.partial_cmp(&two.0).unwrap());
        };

        // break;
        // -TODO write tests to validate the patching of all data structures by regenerating them and then comparing them to the regenerated versions
        // TODO make this a traditional rust test, so I can use the testing framework
        // ?DEBUG TEST
        if TESTING {
            let write_file = File::create("./logs/log").unwrap();
            let mut writer = BufWriter::new(&write_file);

            let (
                t_incident_mesh,
                mut t_vertices,
                t_tris,
                mut t_edges,
                t_collapsible,
                mut t_tri_dict,
                vertex_map,
                tri_list,
            ) = generate_necessary(&incident_mesh, positions);

            // let mesh_positions;
            let mesh_indices;
            let mut vertices_map;
            let tris_set;
            let mut edges_map;
            let collapsible_vec;
            let mut tri_dict_map;

            // compare t_incident_mesh
            {
                mesh_indices =
                    compare_vecs_w_order(&t_incident_mesh.indices, &incident_mesh.indices);
                // mesh_positions =
                //     compare_vecs_w_order(&t_incident_mesh.positions, &incident_mesh.positions);

                if !mesh_indices {
                    write!(writer, "mesh_indices_t: \n{:?} \n", t_incident_mesh.indices).unwrap();
                    write!(
                        writer,
                        "mesh_indices_o: \n{:?} \n\n\n\n\n",
                        incident_mesh.indices
                    )
                    .unwrap();
                }
                // if !mesh_positions {
                //     write!(
                //         writer,
                //         "mesh_positions_t: \n{:?} \n\n\n\n\n",
                //         t_incident_mesh.positions
                //     )
                //     .unwrap();
                //     write!(
                //         writer,
                //         "mesh_positions_o: \n{:?} \n\n\n\n\n",
                //         incident_mesh.positions
                //     )
                //     .unwrap();
                // }
            };

            // compare t_vertices
            {
                vertices_map = true;
                for kv in &vertices {
                    if let Some(value) = t_vertices.get(kv.0) {
                        if !compare_vecs_no_order(kv.1, value) {
                            vertices_map = false;
                            break;
                        }
                    } else {
                        vertices_map = false;
                        break;
                    }

                    t_vertices.remove(kv.0);
                }

                if vertices_map && t_vertices.len() > 0 {
                    vertices_map = false;
                }

                if !vertices_map {
                    let mut t = t_vertices.iter().collect::<Vec<(&u32, &Vec<u32>)>>();
                    t.sort();
                    let mut o = vertices.iter().collect::<Vec<(&u32, &Vec<u32>)>>();
                    o.sort();

                    write!(writer, "vertices_t: \n{:?} \n", t).unwrap();
                    write!(writer, "vertices_o: \n{:?} \n\n\n\n\n", o).unwrap();
                }
            };

            // compare t_tris
            {
                tris_set = t_tris == tris;

                if !tris_set {
                    write!(writer, "tris_t: \n{:?} \n", t_tris).unwrap();
                    write!(writer, "tris_o: \n{:?} \n\n\n\n\n", tris).unwrap();
                }
            };

            // compare t_edges
            {
                edges_map = true;
                for kv in &edges {
                    if let Some(value) = t_edges.get(kv.0) {
                        if kv.1[0] != value[0] || kv.1[1] != value[1] {
                            edges_map = false;
                            break;
                        } else if kv.1[0] == value[1] && kv.1[1] == value[0] {
                            println!("An edge is flipped in the map: {:?}=>{:?}", kv.0, kv.1);
                            edges_map = false;
                            break;
                        }
                    } else {
                        edges_map = false;
                        break;
                    }

                    t_edges.remove(kv.0);
                }

                if edges_map && t_edges.len() > 0 {
                    edges_map = false;
                }

                if !edges_map {
                    let mut t = t_edges.iter().collect::<Vec<(&Edge, &Edge)>>();
                    t.sort();
                    let mut o = edges.iter().collect::<Vec<(&Edge, &Edge)>>();
                    o.sort();

                    write!(writer, "edges_map_t: \n{:?} \n", t).unwrap();
                    write!(writer, "edges_map_o: \n{:?} \n\n\n\n\n", o).unwrap();
                }
            };

            // compare t_collapsible
            {
                let compare = |a: &(f32, Edge), b: &(f32, Edge)| {
                    let cmp = a.0.partial_cmp(&b.0).unwrap();
                    if cmp == Ordering::Equal {
                        let cmp = a.1[0].cmp(&b.1[0]);
                        if cmp == Ordering::Equal {
                            return a.1[1].cmp(&b.1[1]);
                        }
                        return cmp;
                    }

                    cmp
                };
                collapsible_vec = compare_vecs_no_order_customeq(
                    &t_collapsible,
                    &collapsible,
                    &compare,
                    &|a: &(f32, Edge), b: &(f32, Edge)| a.1[0] == b.1[0] && a.1[1] == b.1[1],
                );

                // println!(
                //     "len o: {}, len t: {}",
                //     collapsible.len(),
                //     t_collapsible.len()
                // );

                if !collapsible_vec {
                    let mut t = t_collapsible.clone();
                    t.sort_by(compare);
                    let mut o = collapsible.clone();
                    o.sort_by(compare);

                    write!(writer, "collapsible_vec_t: \n{:?} \n", t).unwrap();
                    write!(writer, "collapsible_vec_o: \n{:?} \n\n\n\n\n", o).unwrap();
                }
            };

            // compare t_tri_dict
            {
                let compare = |a: &Edge, b: &Edge| {
                    let cmp = a.cmp(&b);
                    if cmp == Ordering::Equal {
                        return a[1].cmp(&b[1]);
                    }

                    cmp
                };
                tri_dict_map = true;
                for kv in &tri_dict {
                    if let Some(value) = t_tri_dict.get(kv.0) {
                        if !compare_vecs_no_order_customeq(
                            kv.1,
                            value,
                            &compare,
                            &|a: &Edge, b: &Edge| a[0] == b[0] && a[1] == b[1],
                        ) {
                            tri_dict_map = false;
                            break;
                        }
                    } else {
                        tri_dict_map = false;
                        break;
                    }

                    t_tri_dict.remove(kv.0);
                }

                if tri_dict_map && t_tri_dict.len() > 0 {
                    tri_dict_map = false;
                }

                if !tri_dict_map {
                    let mut t = t_tri_dict
                        .clone()
                        .into_iter()
                        .collect::<Vec<(u32, Vec<Edge>)>>();
                    t.sort();
                    for e in &mut t {
                        e.1.sort();
                    }
                    let mut o = tri_dict
                        .clone()
                        .into_iter()
                        .collect::<Vec<(u32, Vec<Edge>)>>();
                    o.sort();
                    for e in &mut o {
                        e.1.sort();
                    }

                    write!(writer, "tri_dict_t: \n{:?} \n", t).unwrap();
                    write!(writer, "tri_dict_o: \n{:?} \n\n\n\n\n", o).unwrap();
                }
            };

            println!("Test Run");
            println!("mesh_indices: {}", mesh_indices);
            // println!("mesh_positions: {}", mesh_positions);
            println!("vertices_map: {}", vertices_map);
            println!("tris_set: {}", tris_set);
            println!("edges_map: {}", edges_map);
            println!("collapsible_vec: {}", collapsible_vec);
            println!("tri_dict_map: {}", tri_dict_map);
            println!("Test Run Complete \n\n\n");

            if !(
                //mesh_positions &&
                mesh_indices
                    && vertices_map
                    && tris_set
                    && edges_map
                    && collapsible_vec
                    && tri_dict_map
            ) {
                panic!("Test Run Failed");
            }
        }
    }

    // let (indices, positions) = simplify_indices_positions(&incident_mesh.indices, &positions);
    TempMesh {
        indices: incident_mesh.indices,
    }
}

pub fn generate_data_structures(
    indices: &Vec<u32>,
) -> (
    HashMap<Edge, [u32; 2]>,
    HashSet<Triangle>,
    HashMap<u32, Vec<u32>>,
    (HashMap<u32, HashSet<usize>>, Vec<Triangle>),
) {
    let tri_count = indices.len() / 3;
    let mut edges = HashMap::<Edge, [u32; 2]>::with_capacity(tri_count * 24);
    let mut tris = HashSet::<Triangle>::with_capacity(tri_count);
    let mut vertices: HashMap<u32, Vec<u32>> = HashMap::new();

    let mut vertex_map: HashMap<u32, HashSet<usize>> = HashMap::new();
    let mut tri_list: Vec<Triangle> = Vec::with_capacity(indices.len() / 3);

    // indices
    //     .chunks_exact(3)
    //     .map(|tri| [tri[0], tri[1], tri[2]])
    //     .collect();

    // SECTION - Fill vertices, tris, edges and boundary edges
    {
        // * fill vertices, tris and edges
        let mut idx = 0;
        for _ in 0..tri_count {
            let idx0 = indices[idx + 0];
            let idx1 = indices[idx + 1];
            let idx2 = indices[idx + 2];

            let tri_idx = tri_list.len();
            tri_list.push(new_tri(idx0, idx1, idx2));
            if let Some(tlist) = vertex_map.get_mut(&idx0) {
                tlist.insert(tri_idx);
            } else {
                vertex_map.insert(idx0, HashSet::new());
                vertex_map.get_mut(&idx0).unwrap().insert(tri_idx);
            }
            if let Some(tlist) = vertex_map.get_mut(&idx1) {
                tlist.insert(tri_idx);
            } else {
                vertex_map.insert(idx1, HashSet::new());
                vertex_map.get_mut(&idx1).unwrap().insert(tri_idx);
            }
            if let Some(tlist) = vertex_map.get_mut(&idx2) {
                tlist.insert(tri_idx);
            } else {
                vertex_map.insert(idx2, HashSet::new());
                vertex_map.get_mut(&idx2).unwrap().insert(tri_idx);
            }

            {
                let key: Edge = new_edge(idx0, idx1);
                let value = edges.insert(key, [idx2, u32::MAX]);
                if let Some(v) = value {
                    // if v[1] != u32::MAX {
                    //     println!(
                    //         "this edge has appeared more than twice: {:?} => {:?}",
                    //         idx2, v
                    //     );
                    // }
                    // if v[0] == key[0] || v[0] == key[1] {
                    //     println!("something is wrong 1");
                    // }
                    edges.insert(key, new_edge(idx2, v[0]));
                }
            }
            {
                let key: Edge = new_edge(idx0, idx2);
                let value = edges.insert(key, [idx1, u32::MAX]);
                if let Some(v) = value {
                    // if v[1] != u32::MAX {
                    //     println!(
                    //         "this edge has appeared more than twice: {:?} => {:?}",
                    //         idx1, v
                    //     );
                    // }
                    // if v[0] == key[0] || v[0] == key[1] {
                    //     println!("something is wrong 2");
                    // }
                    edges.insert(key, new_edge(idx1, v[0]));
                }
            }
            {
                let key: Edge = new_edge(idx1, idx2);
                let value = edges.insert(key, [idx0, u32::MAX]);
                if let Some(v) = value {
                    // if v[1] != u32::MAX {
                    //     println!(
                    //         "this edge has appeared more than twice: {:?} => {:?}",
                    //         idx0, v
                    //     );
                    // }
                    // if v[0] == key[0] || v[0] == key[1] {
                    //     println!("something is wrong 3");
                    // }
                    edges.insert(key, new_edge(idx0, v[0]));
                }
            }

            {
                vertices
                    .entry(idx0)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx1);
                vertices
                    .entry(idx0)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx2);

                vertices
                    .entry(idx1)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx0);
                vertices
                    .entry(idx1)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx2);

                vertices
                    .entry(idx2)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx0);
                vertices
                    .entry(idx2)
                    .or_insert(Vec::with_capacity(8))
                    .push(idx1);

                // this removes duplicates that happen due to shared verts in triangles
                vertices.entry(idx0).or_default().sort_unstable();
                vertices.entry(idx0).or_default().dedup();
                vertices.entry(idx1).or_default().sort_unstable();
                vertices.entry(idx1).or_default().dedup();
                vertices.entry(idx2).or_default().sort_unstable();
                vertices.entry(idx2).or_default().dedup();
            };

            tris.insert(new_tri(idx0, idx1, idx2));

            idx += 3 as usize;
        }
    };

    for kv in &mut vertices {
        kv.1.sort();
    }

    (edges, tris, vertices, (vertex_map, tri_list))
}

// orders the vertices consistently
pub fn new_tri(a: u32, b: u32, c: u32) -> Triangle {
    if a <= b {
        if b <= c {
            [a, b, c]
        } else if c < a {
            [c, a, b]
        } else {
            [a, c, b]
        }
    } else {
        if a <= c {
            [b, a, c]
        } else if c < b {
            [c, b, a]
        } else {
            [b, c, a]
        }
    }
}

// orders the vertices consistently
pub fn new_edge(a: u32, b: u32) -> Edge {
    if a <= b {
        [a, b]
    } else {
        [b, a]
    }
}

pub fn write_mesh_to_file(name: &str, mesh: &Mesh) {
    let mut f = File::create(name.to_string() + ".obj").unwrap();
    f.write_all(b"mtllib bunny.mtl\no bun_zipper\n").unwrap();
    for v_idx in (0..mesh.positions.len()).step_by(3) {
        f.write_all(
            format!(
                "v {} {} {}\n",
                mesh.positions[v_idx + 0],
                mesh.positions[v_idx + 1],
                mesh.positions[v_idx + 2]
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all(b"usemtl None\ns off\n").unwrap();

    f.write_all(format!("o {}\n", name).as_bytes()).unwrap();
    for idx in (0..mesh.indices.len()).step_by(3) {
        f.write_all(
            format!(
                "f {} {} {}\n",
                mesh.indices[idx + 0] + 1,
                mesh.indices[idx + 1] + 1,
                mesh.indices[idx + 2] + 1
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write_all("\n".as_bytes()).unwrap();

    f.flush().unwrap();
}

pub fn simplify_indices_positions(
    indices: &Vec<u32>,
    positions: &Vec<f32>,
) -> (Vec<u32>, Vec<f32>) {
    let mut mesh_positions = Vec::new();
    let mut mesh_indices = Vec::new();
    {
        let mut existing = HashMap::new();
        let mut idx: u32 = 0;
        for i in indices {
            if let Some(value) = existing.get(i) {
                mesh_indices.push(*value);
            } else {
                existing.insert(i, idx);
                mesh_indices.push(idx);

                mesh_positions.push(positions[*i as usize * 3 + 0]);
                mesh_positions.push(positions[*i as usize * 3 + 1]);
                mesh_positions.push(positions[*i as usize * 3 + 2]);

                idx += 1;
            }
        }
    };

    // DEBUG VALIDATION
    //     {
    //         for i in 0..indices.len() {
    //             let oidx = indices[i];
    //             let nidx = mesh_indices[i];
    //             println!(
    //                 "{}:[{}, {}, {}] => {}:[{}, {}, {}]",
    //                 oidx,
    //                 positions[oidx as usize],
    //                 positions[oidx as usize + 1],
    //                 positions[oidx as usize + 2],
    //                 nidx,
    //                 mesh_positions[nidx as usize],
    //                 mesh_positions[nidx as usize + 1],
    //                 mesh_positions[nidx as usize + 2]
    //             );
    //
    //             if mesh_positions[nidx as usize] != positions[oidx as usize]
    //                 || mesh_positions[nidx as usize + 1] != positions[oidx as usize + 1]
    //                 || mesh_positions[nidx as usize + 2] != positions[oidx as usize + 2]
    //             {
    //                 panic!("indices and position simplification failed!");
    //             }
    //
    //             if i % 3 == 0 {
    //                 println!("");
    //             }
    //         }
    //     }

    (mesh_indices, mesh_positions)
}

pub fn remove_duplicated_vertices(indices: &mut Vec<u32>, positions: &mut Vec<f32>) {
    let mut positions_map = HashMap::<String, u32>::new();
    let mut index_map = HashMap::<u32, u32>::new();
    for idx in (0..positions.len()).step_by(3) {
        let key = format!(
            "[{}, {}, {}]",
            positions[idx + 0],
            positions[idx + 1],
            positions[idx + 2]
        );
        if let Some(new_idx) = positions_map.get(&key) {
            index_map.insert(idx as u32, new_idx.clone());
            continue;
        } else {
            positions_map.insert(key, idx as u32);
        }
    }

    // remove the values at the index_map.keys from positions
    for idx in indices {}
}

pub fn find_equivalent(
    positions_old: &Vec<f32>,
    positions_new: &Vec<f32>,
    idx: &u32,
) -> Option<u32> {
    let x = positions_old[*idx as usize + 0];
    let y = positions_old[*idx as usize + 1];
    let z = positions_old[*idx as usize + 2];

    for idx in (0..positions_new.len()).step_by(3) {
        if x == positions_new[idx + 0] && //// formatting
           y == positions_new[idx + 1] && //// formatting
           z == positions_new[idx + 2]
        {
            return Some(idx as u32);
        }
    }

    None
}

pub fn simplify_indices_positions_cut(
    indices: &Vec<u32>,
    positions: &Vec<f32>,
    cut: &HashSet<Edge>,
) -> (Vec<u32>, Vec<f32>, HashSet<Edge>) {
    let mut mesh_positions = Vec::new();
    let mut mesh_indices = Vec::new();
    let mut mesh_cut = HashSet::new();
    {
        let mut existing = HashMap::new();
        let mut idx: u32 = 0;
        for i in indices {
            if let Some(value) = existing.get(i) {
                mesh_indices.push(*value);
            } else {
                existing.insert(i, idx);
                mesh_indices.push(idx);

                mesh_positions.push(positions[*i as usize * 3 + 0]);
                mesh_positions.push(positions[*i as usize * 3 + 1]);
                mesh_positions.push(positions[*i as usize * 3 + 2]);

                idx += 1;
            }
        }

        for edge in cut {
            mesh_cut.insert(new_edge(
                *existing.get(&edge[0]).unwrap(),
                *existing.get(&edge[1]).unwrap(),
            ));
        }
    };

    // DEBUG VALIDATION NOT FOR CUT THOUGH
    //     {
    //         for i in 0..indices.len() {
    //             let oidx = indices[i];
    //             let nidx = mesh_indices[i];
    //             println!(
    //                 "{}:[{}, {}, {}] => {}:[{}, {}, {}]",
    //                 oidx,
    //                 positions[oidx as usize],
    //                 positions[oidx as usize + 1],
    //                 positions[oidx as usize + 2],
    //                 nidx,
    //                 mesh_positions[nidx as usize],
    //                 mesh_positions[nidx as usize + 1],
    //                 mesh_positions[nidx as usize + 2]
    //             );
    //
    //             if mesh_positions[nidx as usize] != positions[oidx as usize]
    //                 || mesh_positions[nidx as usize + 1] != positions[oidx as usize + 1]
    //                 || mesh_positions[nidx as usize + 2] != positions[oidx as usize + 2]
    //             {
    //                 panic!("indices and position simplification failed!");
    //             }
    //
    //             if i % 3 == 0 {
    //                 println!("");
    //             }
    //         }
    //     }

    (mesh_indices, mesh_positions, mesh_cut)
}

impl Mesh {
    pub fn sqr_dist(&self, a: u32, b: u32) -> f32 {
        let p1 = &self.positions[(a * 3) as usize..(a * 3 + 3) as usize];
        let p2 = &self.positions[(b * 3) as usize..(b * 3 + 3) as usize];

        f32::sqrt((p1[0] - p2[0]).powf(2.0) + (p1[1] - p2[1]).powf(2.0) + (p1[2] - p2[2]).powf(2.0))
    }

    pub fn sqr_dist_w_custom(&self, a: u32, b: Point) -> f32 {
        let p1 = &self.positions[(a * 3) as usize..(a * 3 + 3) as usize];

        f32::sqrt((p1[0] - b[0]).powf(2.0) + (p1[1] - b[1]).powf(2.0) + (p1[2] - b[2]).powf(2.0))
    }

    pub fn sqr_dist_w_2custom(&self, a: Point, b: Point) -> f32 {
        f32::sqrt((a[0] - b[0]).powf(2.0) + (a[1] - b[1]).powf(2.0) + (a[2] - b[2]).powf(2.0))
    }

    pub fn mid_point_2(&self, a: u32, b: u32) -> Point {
        let p1 = &self.positions[(a * 3) as usize..(a * 3 + 3) as usize];
        let p2 = &self.positions[(b * 3) as usize..(b * 3 + 3) as usize];

        [
            (p1[0] + p2[0]) * 0.5,
            (p1[1] + p2[1]) * 0.5,
            (p1[2] + p2[2]) * 0.5,
        ]
    }

    pub fn mid_point_3(&self, a: u32, b: u32, c: u32) -> Point {
        let p1 = &self.positions[(a * 3) as usize..(a * 3 + 3) as usize];
        let p2 = &self.positions[(b * 3) as usize..(b * 3 + 3) as usize];
        let p3 = &self.positions[(c * 3) as usize..(c * 3 + 3) as usize];

        [
            (p1[0] + p2[0] + p3[0]) / 3.0,
            (p1[1] + p2[1] + p3[1]) / 3.0,
            (p1[2] + p2[2] + p3[2]) / 3.0,
        ]
    }

    pub fn mid_point_2custom(a: Point, b: Point) -> Point {
        [
            (a[0] + b[0]) * 0.5,
            (a[1] + b[1]) * 0.5,
            (a[2] + b[2]) * 0.5,
        ]
    }

    pub fn calculate_anchor(&self) -> [f32; 3] {
        let mut sum: Point = [0.0, 0.0, 0.0];

        for idx in (0..self.indices.len()).step_by(3) {
            // TODO make this a vectorized op using a math library
            let center = self.mid_point_3(
                self.indices[idx + 0],
                self.indices[idx + 1],
                self.indices[idx + 2],
            );
            // TODO make this a vectorized op using a math library
            sum[0] += center[0];
            sum[1] += center[1];
            sum[2] += center[2];
        }

        // TODO make this a vectorized op using a math library
        [
            sum[0] / ((self.indices.len() / 3) as f32),
            sum[1] / ((self.indices.len() / 3) as f32),
            sum[2] / ((self.indices.len() / 3) as f32),
        ]
    }
}

pub fn sqr_dist(positions: &Vec<f32>, a: u32, b: u32) -> f32 {
    let p1 = &positions[(a * 3) as usize..(a * 3 + 3) as usize];
    let p2 = &positions[(b * 3) as usize..(b * 3 + 3) as usize];

    f32::sqrt((p1[0] - p2[0]).powf(2.0) + (p1[1] - p2[1]).powf(2.0) + (p1[2] - p2[2]).powf(2.0))
}

pub fn sqr_dist_w_custom(positions: &Vec<f32>, a: u32, b: Point) -> f32 {
    let p1 = &positions[(a * 3) as usize..(a * 3 + 3) as usize];

    f32::sqrt((p1[0] - b[0]).powf(2.0) + (p1[1] - b[1]).powf(2.0) + (p1[2] - b[2]).powf(2.0))
}

pub fn sqr_dist_w_2custom(a: Point, b: Point) -> f32 {
    f32::sqrt((a[0] - b[0]).powf(2.0) + (a[1] - b[1]).powf(2.0) + (a[2] - b[2]).powf(2.0))
}

pub fn mid_point_2(positions: &Vec<f32>, a: u32, b: u32) -> Point {
    let p1 = &positions[(a * 3) as usize..(a * 3 + 3) as usize];
    let p2 = &positions[(b * 3) as usize..(b * 3 + 3) as usize];

    [
        (p1[0] + p2[0]) * 0.5,
        (p1[1] + p2[1]) * 0.5,
        (p1[2] + p2[2]) * 0.5,
    ]
}

pub fn mid_point_3(positions: &Vec<f32>, a: u32, b: u32, c: u32) -> Point {
    let p1 = &positions[(a * 3) as usize..(a * 3 + 3) as usize];
    let p2 = &positions[(b * 3) as usize..(b * 3 + 3) as usize];
    let p3 = &positions[(c * 3) as usize..(c * 3 + 3) as usize];

    [
        (p1[0] + p2[0] + p3[0]) / 3.0,
        (p1[1] + p2[1] + p3[1]) / 3.0,
        (p1[2] + p2[2] + p3[2]) / 3.0,
    ]
}

pub fn mid_point_2custom(a: Point, b: Point) -> Point {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

impl TempMesh {
    pub fn calculate_anchor(&self, positions: &Vec<f32>) -> [f32; 3] {
        let mut sum: Point = [0.0, 0.0, 0.0];

        for idx in (0..self.indices.len()).step_by(3) {
            // TODO make this a vectorized op using a math library
            let center = mid_point_3(
                &positions,
                self.indices[idx + 0],
                self.indices[idx + 1],
                self.indices[idx + 2],
            );
            // TODO make this a vectorized op using a math library
            sum[0] += center[0];
            sum[1] += center[1];
            sum[2] += center[2];
        }

        // TODO make this a vectorized op using a math library
        [
            sum[0] / ((self.indices.len() / 3) as f32),
            sum[1] / ((self.indices.len() / 3) as f32),
            sum[2] / ((self.indices.len() / 3) as f32),
        ]
    }

    pub fn write_mesh_to_file(&self, name: &str, positions: &mut Vec<f32>) {
        let mut f = File::create(name.to_string() + ".obj").unwrap();
        f.write_all(b"mtllib bunny.mtl\no bun_zipper\n").unwrap();
        for v_idx in (0..positions.len()).step_by(3) {
            f.write_all(
                format!(
                    "v {} {} {}\n",
                    positions[v_idx + 0],
                    positions[v_idx + 1],
                    positions[v_idx + 2]
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all(b"usemtl None\ns off\n").unwrap();

        f.write_all(format!("o {}\n", name).as_bytes()).unwrap();
        for idx in (0..self.indices.len()).step_by(3) {
            f.write_all(
                format!(
                    "f {} {} {}\n",
                    self.indices[idx + 0] + 1,
                    self.indices[idx + 1] + 1,
                    self.indices[idx + 2] + 1
                )
                .as_bytes(),
            )
            .unwrap();
        }
        f.write_all("\n".as_bytes()).unwrap();

        f.flush().unwrap();
    }
}
