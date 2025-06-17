use crate::*;
use interop::Vertex;
use std::io::Write;

pub fn dump_clusters(
    src: &[Vertex],
    triangles: &[Triangle],
    clusters: &[AlgoCluster],
    path_addition: impl Into<Option<String>>,
) {
    // Save host visible framebuffer image to disk (ppm format)
    let path = if let Some(addition) = path_addition.into() {
        format!("output/debug_dump{}.obj", addition)
    } else {
        "output/debug_dump.obj".to_string()
    };

    std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        // -
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();

    for vert in src {
        file.write(
            format!(
                "v {} {} {}\n",
                vert.position.x, vert.position.y, vert.position.z
            )
            .as_bytes(),
        );
    }
    file.write(format!("s 0\n",).as_bytes());

    for (idx, cluster) in clusters.iter().enumerate() {
        file.write(format!("\n").as_bytes());
        file.write(format!("o Cluster{}\n", idx).as_bytes());
        for tri_idx in cluster.tri_idx_list {
            if tri_idx == usize::MAX {
                continue;
            }

            let tri = triangles[tri_idx];
            file.write(
                format!(
                    "f {} {} {}\n",
                    tri.idxs[0] + 1,
                    tri.idxs[1] + 1,
                    tri.idxs[2] + 1
                )
                .as_bytes(),
            );
        }
    }

    file.flush();
}

pub fn dump_raw(mesh: &Mesh, path_addition: impl Into<Option<String>>) {
    // Save host visible framebuffer image to disk (ppm format)
    let path = if let Some(addition) = path_addition.into() {
        format!("output/debug_dump{}.obj", addition)
    } else {
        "output/debug_dump.obj".to_string()
    };

    std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        // -
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();

    file.write(format!("o DebugDump\n",).as_bytes());

    for vert in &mesh.vertices {
        file.write(
            format!(
                "v {} {} {}\n",
                vert.position.x, vert.position.y, vert.position.z
            )
            .as_bytes(),
        );
    }

    file.write(format!("s 0\n",).as_bytes());
    for chunk in mesh.indices.chunks(3) {
        file.write(format!("f {} {} {}\n", chunk[0] + 1, chunk[1] + 1, chunk[2] + 1).as_bytes());
    }

    file.flush();
}

pub fn dump_part(
    src: &[Vertex],
    tris: &[Triangle],
    clusters: &Vec<std::ops::Range<usize>>,
    path_addition: impl Into<Option<String>>,
) {
    // Save host visible framebuffer image to disk (ppm format)
    let path = if let Some(addition) = path_addition.into() {
        format!("output/debug_dump{}.obj", addition)
    } else {
        "output/debug_dump.obj".to_string()
    };

    std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        // -
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();

    for vert in src {
        file.write(
            format!(
                "v {} {} {}\n",
                vert.position.x, vert.position.y, vert.position.z
            )
            .as_bytes(),
        );
    }
    file.write(format!("s 0\n",).as_bytes());

    for (cluster_idx, cluster) in clusters.iter().enumerate() {
        file.write(format!("o Cluster{}\n", cluster_idx).as_bytes());
        // file.write(format!("g Cluster{}\n", cluster_idx).as_bytes());
        for tri_idx in cluster.clone() {
            let tri = tris[tri_idx];
            file.write(
                format!(
                    "f {} {} {}\n",
                    tri.idxs[0] + 1,
                    tri.idxs[1] + 1,
                    tri.idxs[2] + 1
                )
                .as_bytes(),
            );
        }
        file.write(format!("\n",).as_bytes());
    }

    file.flush();
}

pub fn dump(
    src: &[Vertex],
    indices: &[u32],
    tris: &[Triangle],
    cluster: &[usize],
    path_addition: impl Into<Option<String>>,
) {
    // Save host visible framebuffer image to disk (ppm format)
    let path = if let Some(addition) = path_addition.into() {
        format!("output/debug_dump{}.obj", addition)
    } else {
        "output/debug_dump.obj".to_string()
    };

    std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        // -
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .unwrap();

    file.write(format!("o DebugDump\n",).as_bytes());

    for vert in src {
        file.write(
            format!(
                "v {} {} {}\n",
                vert.position.x, vert.position.y, vert.position.z
            )
            .as_bytes(),
        );
    }
    file.write(format!("s 0\n",).as_bytes());
    for internal in cluster {
        let tri = tris[*internal];
        let i0 = indices[tri.idx * 3 + 0];
        let i1 = indices[tri.idx * 3 + 1];
        let i2 = indices[tri.idx * 3 + 2];
        file.write(format!("f {} {} {}\n", i0 + 1, i1 + 1, i2 + 1).as_bytes());
    }

    file.flush();
}
