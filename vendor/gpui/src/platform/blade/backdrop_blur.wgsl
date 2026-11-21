// RefineUI patch: real backdrop blur for the Blade renderer.
//
// Three steps per blurred plate, all limited to the plate's sampled region:
//   1. fs_downsample — box-filter the copied frame region down by `scale`
//      (1, 2, 4 or 8) into a float texture.
//   2. fs_blur       — separable Gaussian, run twice (horizontal, vertical).
//   3. fs_composite  — upsample (bilinear), apply saturation, tint (with a
//      blend mode), lighting, grain + dithering and an optional linear mask,
//      clip to the rounded rectangle, draw the drop shadows outside the
//      shape, and blend into the frame.
//
// The composite also runs on its own for a cached blur (same backdrop as the
// previous frame): then `misc.yz` offsets into the cached texture.

struct BlurParams {
    // Rectangle to cover in the render target, in target pixels (x, y, w, h).
    dst_rect: vec4<f32>,
    // Render target size (w, h, _, _).
    target_size: vec4<f32>,
    // Source texture size (w, h, 1 / w, 1 / h).
    src_size: vec4<f32>,
    // Valid source texels, in source pixels (x0, y0, x1, y1).
    src_clamp: vec4<f32>,
    // (scale, sigma, dir.x, dir.y): downsample factor / Gaussian sigma in
    // source texels / blur direction.
    params: vec4<f32>,
    // Plate shape in target pixels (x, y, w, h).
    shape: vec4<f32>,
    // Corner radii (top-left, top-right, bottom-right, bottom-left).
    radii: vec4<f32>,
    // Tint, linear RGBA (straight alpha).
    tint: vec4<f32>,
    // (saturation, opacity, premultiplied output, downsample factor).
    extra: vec4<f32>,
    // (direction towards the light x, y, directional, ambient).
    light_dir: vec4<f32>,
    // (edge, edge width px, specular, specular size px).
    light_a: vec4<f32>,
    // (specular position x, y (shape-relative), reflection, noise).
    light_b: vec4<f32>,
    // (light r, g, b, tint blend mode).
    light_color: vec4<f32>,
    // Linear mask (dir.x, dir.y, start, end); zero direction = off.
    mask: vec4<f32>,
    // (shadow count, source offset x, y, _) — offset in downsampled texels.
    misc: vec4<f32>,
    // Per shadow: (offset x, y, blur sigma, spread), then color (straight RGBA).
    shadows: array<vec4<f32>, 8>,
}

var<uniform> blur_params: BlurParams;
var t_blur: texture_2d<f32>;
var s_blur: sampler;

const BLUR_LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

@vertex
fn vs_blur(@builtin(vertex_index) vertex_id: u32) -> @builtin(position) vec4<f32> {
    let unit = vec2<f32>(f32(vertex_id & 1u), f32((vertex_id >> 1u) & 1u));
    let position = blur_params.dst_rect.xy + unit * blur_params.dst_rect.zw;
    let ndc = position / blur_params.target_size.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    return vec4<f32>(ndc, 0.0, 1.0);
}

// Sample at a continuous source position (texel centers at +0.5), clamped
// to the valid region so the kernel never reads stale texels.
fn sample_clamped(position: vec2<f32>) -> vec4<f32> {
    let p = clamp(position, blur_params.src_clamp.xy + vec2<f32>(0.5), blur_params.src_clamp.zw - vec2<f32>(0.5));
    return textureSampleLevel(t_blur, s_blur, p * blur_params.src_size.zw, 0.0);
}

@fragment
fn fs_downsample(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let scale = blur_params.params.x;
    let center = frag.xy * scale;
    if (scale < 2.0) {
        return sample_clamped(center);
    }
    // Each bilinear tap placed between four texels averages them, so an
    // n × n grid of taps two texels apart averages the scale × scale block.
    let n = i32(scale * 0.5);
    var acc = vec4<f32>(0.0);
    for (var j = 0; j < n; j += 1) {
        for (var i = 0; i < n; i += 1) {
            let offset = vec2<f32>(f32(i) * 2.0 + 1.0, f32(j) * 2.0 + 1.0) - vec2<f32>(scale * 0.5);
            acc += sample_clamped(center + offset);
        }
    }
    return acc / f32(n * n);
}

@fragment
fn fs_blur(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let sigma = max(blur_params.params.y, 0.0001);
    let dir = blur_params.params.zw;
    let taps = i32(min(ceil(sigma * 3.0), 64.0));
    let k = -0.5 / (sigma * sigma);
    var acc = sample_clamped(frag.xy);
    var total = 1.0;
    for (var i = 1; i <= taps; i += 1) {
        let x = f32(i);
        let w = exp(x * x * k);
        acc += w * (sample_clamped(frag.xy + dir * x) + sample_clamped(frag.xy - dir * x));
        total += 2.0 * w;
    }
    return acc / total;
}

fn blur_corner_radius(center_to_point: vec2<f32>, radii: vec4<f32>) -> f32 {
    if (center_to_point.x < 0.0) {
        if (center_to_point.y < 0.0) {
            return radii.x;
        }
        return radii.w;
    }
    if (center_to_point.y < 0.0) {
        return radii.y;
    }
    return radii.z;
}

// Signed distance to a rounded rectangle (x, y, w, h) — negative inside.
fn rrect_distance(position: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let half_size = rect.zw * 0.5;
    let to_point = position - (rect.xy + half_size);
    let radius = min(blur_corner_radius(to_point, radii), min(half_size.x, half_size.y));
    let q = abs(to_point) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Outward normal of the rounded rectangle at the closest edge.
fn rrect_normal(position: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> vec2<f32> {
    let half_size = rect.zw * 0.5;
    let to_point = position - (rect.xy + half_size);
    let radius = min(blur_corner_radius(to_point, radii), min(half_size.x, half_size.y));
    let q = abs(to_point) - half_size + vec2<f32>(radius);
    let s = select(vec2<f32>(-1.0), vec2<f32>(1.0), to_point >= vec2<f32>(0.0));
    if (q.x > 0.0 && q.y > 0.0) {
        return normalize(q) * s;
    }
    if (q.x > q.y) {
        return vec2<f32>(s.x, 0.0);
    }
    return vec2<f32>(0.0, s.y);
}

fn blur_gaussian_1d(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * 3.14159265) * sigma);
}

fn blur_erf(v: vec2<f32>) -> vec2<f32> {
    let s = sign(v);
    let a = abs(v);
    let r1 = 1.0 + (0.278393 + (0.230389 + (0.000972 + 0.078108 * a) * a) * a) * a;
    let r2 = r1 * r1;
    return s - s / (r2 * r2);
}

fn blur_shadow_along_x(x: f32, y: f32, sigma: f32, corner: f32, half_size: vec2<f32>) -> f32 {
    let delta = min(half_size.y - corner - abs(y), 0.0);
    let curved = half_size.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    let integral = 0.5 + 0.5 * blur_erf((x + vec2<f32>(-curved, curved)) * (sqrt(0.5) / sigma));
    return integral.y - integral.x;
}

// Coverage of a blurred rounded rectangle (same model as gpui's shadows).
fn shadow_alpha(position: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, sigma: f32) -> f32 {
    if (sigma < 0.5) {
        return clamp(0.5 - rrect_distance(position, rect, radii), 0.0, 1.0);
    }
    let half_size = rect.zw * 0.5;
    let to_point = position - (rect.xy + half_size);
    let corner = min(blur_corner_radius(to_point, radii), min(half_size.x, half_size.y));
    let low = to_point.y - half_size.y;
    let high = to_point.y + half_size.y;
    let start = clamp(-3.0 * sigma, low, high);
    let end = clamp(3.0 * sigma, low, high);
    let step = (end - start) / 4.0;
    var y = start + step * 0.5;
    var alpha = 0.0;
    for (var i = 0; i < 4; i += 1) {
        alpha += blur_shadow_along_x(to_point.x, to_point.y - y, sigma, corner, half_size)
            * blur_gaussian_1d(y, sigma) * step;
        y += step;
    }
    return clamp(alpha, 0.0, 1.0);
}

fn blur_hash(p: vec2<f32>, seed: u32) -> f32 {
    var x = u32(p.x) * 1664525u + u32(p.y) * 1013904223u + seed * 2654435761u;
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return f32(x) * (1.0 / 4294967295.0);
}

fn screen3(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(1.0) - (vec3<f32>(1.0) - a) * (vec3<f32>(1.0) - b);
}

fn blend_tint(base: vec3<f32>, tint: vec3<f32>, mode: f32) -> vec3<f32> {
    if (mode < 0.5) {
        return tint;
    }
    if (mode < 1.5) {
        return base * tint;
    }
    if (mode < 2.5) {
        return screen3(base, tint);
    }
    if (mode < 3.5) {
        let low = 2.0 * base * tint;
        let high = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - base) * (vec3<f32>(1.0) - tint);
        return select(high, low, base <= vec3<f32>(0.5));
    }
    // W3C soft-light.
    let d = select(sqrt(base), ((16.0 * base - 12.0) * base + 4.0) * base, base <= vec3<f32>(0.25));
    let soft_low = base - (vec3<f32>(1.0) - 2.0 * tint) * base * (vec3<f32>(1.0) - base);
    let soft_high = base + (2.0 * tint - vec3<f32>(1.0)) * (d - base);
    return select(soft_high, soft_low, tint <= vec3<f32>(0.5));
}

@fragment
fn fs_composite(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let p = blur_params;
    let shape = p.shape;
    let distance = rrect_distance(frag.xy, shape, p.radii);
    let coverage = clamp(0.5 - distance, 0.0, 1.0);
    let opacity = p.extra.y;

    var plate = vec4<f32>(0.0);
    if (coverage > 0.0) {
        let ds = p.extra.w;
        let sampled = sample_clamped(frag.xy / ds - p.misc.yz);
        let alpha = sampled.a;
        var rgb = sampled.rgb / max(alpha, 0.0001);

        // Saturation.
        let luma = dot(rgb, BLUR_LUMA);
        rgb = max(mix(vec3<f32>(luma), rgb, p.extra.x), vec3<f32>(0.0));

        // Tint with blend mode (alpha = amount).
        let tint = p.tint;
        rgb = mix(rgb, blend_tint(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), tint.rgb, p.light_color.w), tint.a);
        let out_alpha = alpha + tint.a * (1.0 - alpha);

        // Lighting.
        let light = p.light_color.rgb;
        let dir = p.light_dir.xy;
        let half_size = shape.zw * 0.5;
        let local = (frag.xy - (shape.xy + half_size)) / max(half_size, vec2<f32>(1.0));
        let has_dir = dot(dir, dir) > 0.0;
        if (p.light_dir.z > 0.0 && has_dir) {
            let g = clamp(dot(local, dir), -1.0, 1.0);
            let lit = p.light_dir.z * smoothstep(-0.2, 1.0, g);
            rgb = screen3(rgb, light * lit);
            rgb = rgb * (1.0 - p.light_dir.z * 0.35 * smoothstep(0.0, 1.0, -g));
        }
        if (p.light_dir.w > 0.0) {
            rgb = screen3(rgb, light * p.light_dir.w);
        }
        if (p.light_b.z > 0.0) {
            // Glossy reflection: a soft diagonal band on the lit side.
            var r = vec2<f32>(0.70710677, -0.70710677);
            if (has_dir) {
                r = normalize(dir + vec2<f32>(-dir.y, dir.x));
            }
            let t = dot(local, r);
            let u = (t - 0.42) / 0.26;
            let band = exp(-u * u);
            rgb = screen3(rgb, light * (p.light_b.z * band));
        }
        if (p.light_a.z > 0.0) {
            let spot = shape.xy + p.light_b.xy * shape.zw;
            let size = max(p.light_a.w, 1.0);
            let d = frag.xy - spot;
            let spec = exp(-dot(d, d) / (2.0 * size * size));
            rgb = screen3(rgb, light * (p.light_a.z * spec));
        }
        if (p.light_a.x > 0.0) {
            let width = max(p.light_a.y, 0.5);
            let rim = exp(-max(-distance, 0.0) / width);
            var facing = 1.0;
            if (has_dir) {
                let n = rrect_normal(frag.xy, shape, p.radii);
                facing = 0.3 + 0.7 * max(dot(n, dir), 0.0);
            }
            rgb = screen3(rgb, light * (p.light_a.x * rim * facing));
        }

        // Grain, then a ±0.5 LSB dither so 8-bit output does not band.
        let grain = (blur_hash(frag.xy, 1u) - 0.5) * 2.0 * p.light_b.w;
        let dither = (blur_hash(frag.xy, 2u) - blur_hash(frag.xy, 3u)) / 255.0;
        rgb = clamp(rgb + vec3<f32>(grain + dither), vec3<f32>(0.0), vec3<f32>(1.0));

        // Linear mask.
        var mask = 1.0;
        if (dot(p.mask.xy, p.mask.xy) > 0.0) {
            let t = dot((frag.xy - shape.xy) / max(shape.zw, vec2<f32>(1.0)) - vec2<f32>(0.5), p.mask.xy) + 0.5;
            mask = clamp((p.mask.w - t) / max(p.mask.w - p.mask.z, 0.0001), 0.0, 1.0);
        }
        let a = out_alpha * coverage * opacity * mask;
        plate = vec4<f32>(rgb * a, a);
    }

    // Drop shadows, outside the shape only.
    var shadow = vec4<f32>(0.0);
    let count = i32(p.misc.x);
    for (var i = 0; i < 4; i += 1) {
        if (i >= count) {
            break;
        }
        let geometry = p.shadows[i * 2];
        let color = p.shadows[i * 2 + 1];
        let spread = geometry.w;
        let rect = vec4<f32>(shape.xy + geometry.xy - vec2<f32>(spread), shape.zw + vec2<f32>(spread * 2.0));
        let radii = max(p.radii + vec4<f32>(spread), vec4<f32>(0.0));
        let a = shadow_alpha(frag.xy, rect, radii, geometry.z) * color.a;
        shadow = vec4<f32>(color.rgb * a, a) + shadow * (1.0 - a);
    }
    shadow = shadow * ((1.0 - coverage) * opacity);

    let result = plate + shadow * (1.0 - plate.a);
    if (p.extra.z > 0.5) {
        return result;
    }
    return vec4<f32>(result.rgb / max(result.a, 0.0001), result.a);
}
