use interop::{Cluster, Node};
use std::fs::File;
use std::mem;
use std::io::Seek;
use std::io;
use std::io::prelude::*;

pub struct CTree {
    pub nodes: Vec<Node>,
    pub cluster_cache: Vec<Cluster>,
    pub roots: [u32; 2],
    file: File,
}

pub fn read(file_name: &str) -> CTree {
    let mut read_file = File::open(file_name).unwrap();

    let mut offset = 0;
    let nodes_length = read_u32_offset(&mut read_file, &mut offset) as usize;

    // // TODO TEMP
    // {
    //     let mut offset_copy = offset;
    //     let first_node_offset = read_u32_offset(&mut read_file, &mut offset_copy);
    //     println!("First node offset: {}", first_node_offset);
    // };

    // read_file
    //     .seek(std::io::SeekFrom::Start(mem::size_of::<u32>() as u64))
    //     .unwrap();

    // let mut buffer = Vec::<u8>::with_capacity(mem::size_of::<Node>() * nodes_length as usize);
    // unsafe { buffer.set_len(mem::size_of::<Node>() * nodes_length as usize) };
    // read_file.read_exact(&mut buffer).unwrap();
    // let ptr = buffer.as_mut_ptr();
    // // let cap = buffer.capacity();
    // // let len = buffer.len();
    // mem::forget(buffer); // Avoid calling the destructor!

    // println!("Size of Node: {}", mem::size_of::<Node>());
    let nodes: Vec<Node> =
        // unsafe { Vec::from_raw_parts(ptr as *mut Node, nodes_length, nodes_length) };
        read_vec::<Node>(&mut read_file, nodes_length, &mut offset);

    // let mut u32_buff = [0 as u8; mem::size_of::<u32>()];
    // read_file.read_exact(&mut u32_buff).unwrap();
    // let clusters_length = u32::from_le_bytes(u32_buff) as usize;

    // println!("clusters_length: {}", nodes_length);

    let mut cluster_cache = Vec::with_capacity(nodes_length);
    cluster_cache.resize(
        nodes_length,
        Cluster {
            pos: Vec::with_capacity(0),
            idx: vec![],
        },
    );

    // TODO find the upper most parents?
    let roots = [0 as u32; 2];

    CTree {
        nodes,
        cluster_cache,
        roots,
        file: read_file,
    }
}

impl CTree {
    fn load_cluster(&mut self, id: &u32) -> Cluster {
        let mut offset = self.nodes[*id as usize].offset as u64;
        // println!("offset before: {}", offset);
        let positions = {
            let positions_length = read_u32_offset(&mut self.file, &mut offset) as usize;
            // println!("positions_length: {}", positions_length);
            read_vec::<f32>(&mut self.file, positions_length, &mut offset)

            // let buff_size = mem::size_of::<f32>() * positions_length;
            // let mut buffer = Vec::<u8>::with_capacity(buff_size);
            // unsafe { buffer.set_len(buff_size) };
            // self.file.seek(SeekFrom::Start(offset)).unwrap();
            // self.file.read_exact(&mut buffer).unwrap();
            // offset += buff_size as u64;
            // let ptr = buffer.as_mut_ptr();
            // mem::forget(buffer); // Avoid calling the destructor!
            //
            // unsafe {
            //     Vec::from_raw_parts(ptr as *mut f32, positions_length, positions_length)
            // }
        };

        let indices = {
            let indices_length = read_u32_offset(&mut self.file, &mut offset) as usize;
            // println!("indices_length: {}", indices_length);
            read_vec::<u32>(&mut self.file, indices_length, &mut offset)

            // let buff_size = mem::size_of::<u32>() * indices_length;
            // let mut buffer = Vec::<u8>::with_capacity(buff_size);
            // unsafe { buffer.set_len(buff_size) };
            // self.file.seek(SeekFrom::Start(offset)).unwrap();
            // self.file.read_exact(&mut buffer).unwrap();
            // // offset += buff_size as u64;
            // let ptr = buffer.as_mut_ptr();
            // mem::forget(buffer); // Avoid calling the destructor!
            //
            // unsafe { Vec::from_raw_parts(ptr as *mut u32, indices_length, indices_length) }
        };

        // println!("offset after: {}", offset);

        Cluster {
            pos: positions,
            idx: indices,
        }
    }

    pub fn get_cluster(&mut self, id: &u32) -> Option<&Cluster> {
        // println!("cluster_cache len: {}", self.cluster_cache.len());
        if *id < self.cluster_cache.len() as u32 {
            if !self.cluster_cache[*id as usize].pos.is_empty() {
                Some(&self.cluster_cache[*id as usize])
            } else {
                let read_cluster = self.load_cluster(id);

                self.cluster_cache[*id as usize] = read_cluster;
                Some(&self.cluster_cache[*id as usize])
            }
        } else {
            None
        }
    }

    pub fn get_node(&self, id: &usize) -> &Node {
        &self.nodes[*id]
    }

    pub fn get_cluster_w_offset(&mut self, id: &u32) -> Option<(&Cluster, u32)> {
        // println!("cluster_cache len: {}", self.cluster_cache.len());
        if *id < self.cluster_cache.len() as u32 {
            if !self.cluster_cache[*id as usize].pos.is_empty() {
                Some((
                    &self.cluster_cache[*id as usize],
                    self.nodes[*id as usize].offset,
                ))
            } else {
                let read_cluster = self.load_cluster(id);

                self.cluster_cache[*id as usize] = read_cluster;
                Some((
                    &self.cluster_cache[*id as usize],
                    self.nodes[*id as usize].offset,
                ))
            }
        } else {
            None
        }
    }

    // TODO(@Max): Make it so that this loads the entire file contents into memory then just do a vec from raw parts with ptrs and offsets, need to figure out what to do with the length value in the memory
    // fill_cache, cache_all
    pub fn load_all(&mut self) {
        for idx in 0..self.cluster_cache.len() {
            if self.cluster_cache[idx as usize].pos.is_empty() {
                let read_cluster = self.load_cluster(&(idx as u32));
                self.cluster_cache[idx as usize] = read_cluster;
            }
        }
    }

    // TODO this likely should not exist but its here for the intial debugging
    pub fn iter_all(&self) -> std::slice::Iter<Cluster> {
        self.cluster_cache.iter()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

// fn read_u32(file: &mut File) -> u32 {
//     let mut u32_buff = [0 as u8; mem::size_of::<u32>()];
//     file.read_exact(&mut u32_buff).unwrap();
//     u32::from_le_bytes(u32_buff)
// }

fn read_u32_offset(file: &mut File, offset: &mut u64) -> u32 {
    let mut u32_buff = [0 as u8; mem::size_of::<u32>()];
    let _ = file.seek(std::io::SeekFrom::Start(*offset));
    let _ = file.read_exact(&mut u32_buff);
    *offset += mem::size_of::<u32>() as u64;
    u32::from_le_bytes(u32_buff)
}

fn read_vec<T>(file: &mut File, length: usize, offset: &mut u64) -> Vec<T> {
    // println!("Size of T: {}", mem::size_of::<T>());

    let buff_size = mem::size_of::<T>() * length;
    let mut buffer = Vec::<u8>::with_capacity(buff_size);
    buffer.resize(buff_size, Default::default());
    let _ = file.seek(std::io::SeekFrom::Start(*offset));
    let _ = file.read_exact(&mut buffer);
    *offset += buff_size as u64;
    let ptr = buffer.as_mut_ptr();
    mem::forget(buffer); // Avoid calling the destructor!
    unsafe { Vec::from_raw_parts(ptr as *mut T, length, length) }
}
