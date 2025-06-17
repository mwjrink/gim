@group(0) @binding(0)
var<storage, read> tris: array<vec3<f32>>;
@group(0) @binding(1)
var<storage, read> seeds: array<vec3<f32>>;
@group(0) @binding(2)
var<storage, read_write> closest_seed: array<u64>;

@group(1) @binding(0)
var<uniform, read> seeds_size: u32;
@group(1) @binding(1)
var<uniform, read> tris_size: u32;

@compute @workgroup_size(64)
fn nearest(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var tri_pos = tris[global_id.x];
    var min_dist = 3.40282347E+38f32;
    var seed_idx = 0;
    for (var i = 0; i < seeds_size; i += 1) {
        var dist_vec = (seeds[i] - tri_pos);
        var sq_dist = dist_vec.x * dist_vec.x + dist_vec.y * dist_vec.y + dist_vec.z * dist_vec.z;
        if sq_dist < min_dist {
            seed_idx = i;
        }
    }
    closest_seed[global_id.x] = seed_idx;
}
