use std::io::Write;

use crate::writer::Triangle;
use interop::Vertex;

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
        file.write(format!("f {} {} {}\n", i0, i1, i2).as_bytes());
    }

    file.flush();
}
