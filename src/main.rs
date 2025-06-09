use cmwriter::writer::{self, Mesh};
use interop::Vertex;
use ultraviolet::{Vec2, Vec3};

fn main() {
    let obj_file = "input/lucy/lucy.obj";

    let (models, materials) =
        tobj::load_obj(obj_file, &tobj::GPU_LOAD_OPTIONS).expect("Failed to load file");

    println!("# of models: {}", models.len());

    if let Ok(mats) = materials {
        println!("# of materials: {}", mats.len());
    }

    for (i, m) in models.iter().enumerate() {
        let mesh = &m.mesh;
        println!("model[{}].name = \'{}\'", i, m.name);
        println!(
            "model[{}].mesh.material_id = {:?}",
            i,
            mesh.material_id.unwrap_or_else(|| { usize::MAX })
        );

        println!("Tris in model[{}]: {}", i, mesh.face_arities.len());
        println!("Number of verts: {}", mesh.positions.len());

        let mut vertices = Vec::with_capacity(mesh.positions.len() / 3);
        for _idx in 0..(mesh.positions.len() / 3) {
            let idx = _idx * 3;
            vertices.push(Vertex {
                position: Vec3::new(
                    mesh.positions[idx + 0],
                    mesh.positions[idx + 1],
                    mesh.positions[idx + 2],
                ),
                normal: Vec3::new(
                    mesh.normals[idx + 0],
                    mesh.normals[idx + 1],
                    mesh.normals[idx + 2],
                ),
                tex_coords: Vec2::new(
                    // -
                    // mesh.texcoords[idx + 0],
                    // mesh.texcoords[idx + 1],
                    0.0,
                    0.0, // lucy doesn't have a texture, make sure to fix this in the future
                ),
            });
        }

        // TODO probably bad cause these could be HUGE, do something else, ...
        // TODO... but that may have to wait for a better asset importer or a switch to gltf2 or usda
        let mut mesh = Mesh {
            vertices: vertices,
            indices: mesh.indices.clone(),
        };

        println!("About to write.");
        let output = writer::write(&mut mesh);
        println!("Done write.");

        // let mut read = reader::read("output/test_output.cm");
        // println!("read len: {}\nout len: {}\n\n", read.len(), output.len());

        // for idx in 0..output.len() {
        //     let onode = output.get_node(&idx);
        //     let rnode = read.get_node(&idx);

        //     // println!("offset      onode: {} | rnode: {}", onode.offset,      &rnode.offset);
        //     // println!("children[0] onode: {} | rnode: {}", onode.children[0], &rnode.children[0]);
        //     // println!("children[1] onode: {} | rnode: {}", onode.children[1], &rnode.children[1]);
        //     // println!("children[2] onode: {} | rnode: {}", onode.children[2], &rnode.children[2]);
        //     // println!("children[3] onode: {} | rnode: {}", onode.children[3], &rnode.children[3]);

        //     assert!(onode.offset == rnode.offset);
        //     assert!(onode.children[0] == rnode.children[0]);
        //     assert!(onode.children[1] == rnode.children[1]);
        //     assert!(onode.children[2] == rnode.children[2]);
        //     assert!(onode.children[3] == rnode.children[3]);
        // }

        // for idx in 0..output.len() {
        //     let outp_cluster = output.get(&(idx as u32));
        //     // println!(
        //     //     "outp_cluster.mesh.positions len: {}",
        //     //     outp_cluster.mesh.positions.len()
        //     // );
        //     // println!(
        //     //     "outp_cluster.mesh.indices len: {}",
        //     //     outp_cluster.mesh.indices.len()
        //     // );

        //     let read_cluster = read.get_cluster(&(idx as u32)).unwrap();
        //     // println!("read_offset: {}, out_offset: {}", roffset, ooffset);
        //     if read_cluster.pos.is_empty() || read_cluster.idx.is_empty() {
        //         println!("read pos len: {}", read_cluster.pos.len());
        //         println!("read idx len: {}", read_cluster.idx.len());
        //     }
        //     // println!("read_cluster.idx: {}", read_cluster.idx.len());

        //     let idx_equal = compare_vecs_w_order(&read_cluster.pos, &outp_cluster.mesh.positions);
        //     let pos_equal = compare_vecs_w_order(&read_cluster.idx, &outp_cluster.mesh.indices);

        //     // println!("pos: {}\nidx: {}\n\n", pos_equal, idx_equal);

        //     // println!("roffset: {}", roffset);
        //     // println!("ooffset: {}", ooffset);
        //     // This is already checked in the previous loop
        //     // assert!(roffset == ooffset as u32);
        //     assert!(pos_equal);
        //     assert!(idx_equal);
        // }

        // println!("SUCCESS, The input and output are identical!")
    }

    return ();
}
