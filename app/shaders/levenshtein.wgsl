struct InputBuffer {
    data: array<u32>,
};

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: array<u32>;

const MAX_CHARS: u32 = 64u;
const STRIDE: u32 = 130u;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&output_buf)) {
        return;
    }
    let base = idx * STRIDE;
    let la = min(input_buf.data[base], MAX_CHARS);
    let lb = min(input_buf.data[base + 1u], MAX_CHARS);

    if (la == 0u && lb == 0u) {
        output_buf[idx] = 10000u;
        return;
    }
    if (la == 0u || lb == 0u) {
        output_buf[idx] = 0u;
        return;
    }

    var prev: array<u32, 65>;
    var curr: array<u32, 65>;
    var j: u32 = 0u;
    loop {
        if (j > lb) { break; }
        prev[j] = j;
        j = j + 1u;
    }

    var i: u32 = 1u;
    loop {
        if (i > la) { break; }
        curr[0] = i;
        j = 1u;
        loop {
            if (j > lb) { break; }
            let ca = input_buf.data[base + 2u + (i - 1u)];
            let cb = input_buf.data[base + 66u + (j - 1u)];
            let cost = select(1u, 0u, ca == cb);
            let deletion = prev[j] + 1u;
            let insertion = curr[j - 1u] + 1u;
            let substitution = prev[j - 1u] + cost;
            curr[j] = min(deletion, min(insertion, substitution));
            j = j + 1u;
        }
        j = 0u;
        loop {
            if (j > lb) { break; }
            prev[j] = curr[j];
            j = j + 1u;
        }
        i = i + 1u;
    }

    let dist = prev[lb];
    let max_len = max(la, lb);
    let similarity = clamp(1.0 - f32(dist) / f32(max_len), 0.0, 1.0);
    output_buf[idx] = u32(similarity * 10000.0 + 0.5);
}
