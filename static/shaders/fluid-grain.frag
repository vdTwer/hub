#version 300 es
// Fluid, on film.
//
// Two passes in one shader:
//   1. A clean, slow gradient: low-octave gradient noise, domain-warped twice
//      so colors fold into each other like dye in water.
//   2. An analog print of it: 24 fps grain, gate weave, exposure flicker,
//      halation, a film tone curve, vignette, and the odd speck of dust.
// u_film blends pass 2 in and out, so the two can be compared live.
//
// The stage runs this shader twice per frame. With u_pass == 0 it writes
// the highlights of the clean image into a small, mipmapped texture; with
// u_pass == 1 it draws the frame and reads that texture back, blurred, for
// halation. The pointer trail in u_stir lets the viewer stir the fluid.
precision highp float;

uniform vec2 u_res;
uniform float u_time;
uniform float u_film;
uniform int u_pass;
uniform sampler2D u_scene;

// Recent pointer samples, newest first: xy position and zw velocity, both in
// the same units as uv below. A negative age marks an unused slot.
const int STIR = 12;
uniform vec4 u_stir[STIR];
uniform float u_stirAge[STIR];

out vec4 fragColor;

// ---------------------------------------------------------------- hashing

// PCG hash (Jarzynski and Olano, 2020): cheap and free of visible patterns,
// which matters for grain more than anything else here.
uint pcg(uint v) {
    uint s = v * 747796405u + 2891336453u;
    uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

float rand(uvec3 v) {
    return float(pcg(v.x ^ pcg(v.y ^ pcg(v.z)))) * (1.0 / 4294967296.0);
}

// --------------------------------------------------------------- gradient

vec2 gradDir(ivec2 cell) {
    float a = rand(uvec3(uvec2(cell), 17u)) * 6.2831853;
    return vec2(cos(a), sin(a));
}

// Gradient noise with quintic fade, roughly in [-1, 1].
float gnoise(vec2 p) {
    ivec2 i = ivec2(floor(p));
    vec2 f = fract(p);
    vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float a = dot(gradDir(i), f);
    float b = dot(gradDir(i + ivec2(1, 0)), f - vec2(1, 0));
    float c = dot(gradDir(i + ivec2(0, 1)), f - vec2(0, 1));
    float d = dot(gradDir(i + ivec2(1, 1)), f - vec2(1, 1));
    return 1.4 * mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Three octaves only: more detail would read as smoke, not as silk.
float fbm(vec2 p) {
    const mat2 turn = mat2(0.8, 0.6, -0.6, 0.8);
    float sum = 0.0, amp = 0.55;
    for (int i = 0; i < 3; i++) {
        sum += amp * gnoise(p);
        p = turn * p * 2.03 + 11.7;
        amp *= 0.45;
    }
    return sum;
}

vec3 srgbToLinear(vec3 c) { return pow(c, vec3(2.2)); }

// How far the pointer has dragged the dye at uv. Each sample pushes along
// its velocity and swirls around its path, then relaxes over a few seconds,
// so a stroke leaves a wake that slowly heals.
vec2 stir(vec2 uv) {
    vec2 offset = vec2(0.0);
    for (int i = 0; i < STIR; i++) {
        float age = u_stirAge[i];
        if (age < 0.0) continue;
        vec2 d = uv - u_stir[i].xy;
        vec2 vel = u_stir[i].zw;
        float speed = length(vel);
        if (speed > 2.5) vel *= 2.5 / speed;
        float reach = 0.035 + 0.05 * age;            // the wake widens as it fades
        float w = exp(-dot(d, d) / reach) * exp(-1.2 * age);
        offset += w * (0.09 * vel + 0.18 * min(speed, 2.5) * vec2(-d.y, d.x));
    }
    return offset;
}

// The clean image, in linear light.
vec3 fluid(vec2 uv, float t) {
    vec2 p = (uv - stir(uv)) * 0.7;

    // Two rounds of domain warping, each drifting at its own pace.
    vec2 q = vec2(fbm(p + vec2(0.0, 0.11 * t)),
                  fbm(p + vec2(5.2, 1.3) - 0.09 * t));
    vec2 r = vec2(fbm(p + 1.25 * q + vec2(1.7, 9.2) + 0.07 * t),
                  fbm(p + 1.25 * q + vec2(8.3, 2.8) - 0.06 * t));
    float f = fbm(p + 1.1 * r + 0.03 * t);

    vec3 paper   = srgbToLinear(vec3(0.94, 0.93, 0.95));
    vec3 cobalt  = srgbToLinear(vec3(0.16, 0.27, 0.85));
    vec3 lilac   = srgbToLinear(vec3(0.72, 0.62, 0.98));
    vec3 apricot = srgbToLinear(vec3(1.00, 0.60, 0.36));
    vec3 lagoon  = srgbToLinear(vec3(0.20, 0.72, 0.78));

    vec3 col = mix(paper, lilac, 0.9 * smoothstep(-0.3, 0.4, f));
    col = mix(col, lagoon, 0.9 * smoothstep(-0.1, 0.55, q.y - 0.4 * r.x));
    col = mix(col, cobalt, smoothstep(0.0, 0.7, r.x + 0.35 * f));
    col = mix(col, apricot, 0.95 * smoothstep(0.05, 0.7, -r.y + 0.5 * q.x));
    col = mix(col, paper, 0.5 * smoothstep(0.3, 0.95, -f - 0.5 * q.y));
    return col;
}

// ------------------------------------------------------------------- film

float luma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

// Unit-variance, roughly Gaussian value at a lattice point (sum of three
// uniforms), fresh every frame.
float gaussAt(ivec2 cell, uint frame) {
    uvec3 k = uvec3(uvec2(cell), frame);
    float s = rand(k) + rand(k + uvec3(0u, 0u, 7919u)) + rand(k + uvec3(0u, 0u, 15887u));
    return 2.0 * (s - 1.5);
}

// Grain is smoothly interpolated noise, so it clumps like silver
// instead of buzzing like digital noise.
float grainLayer(vec2 p, uint frame) {
    ivec2 i = ivec2(floor(p));
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(gaussAt(i, frame), gaussAt(i + ivec2(1, 0), frame), f.x),
               mix(gaussAt(i + ivec2(0, 1), frame), gaussAt(i + ivec2(1, 1), frame), f.x), f.y);
}

// A soft S-curve with lifted blacks and a gentle shoulder, plus a faint
// split tone: cool shadows, warm highlights.
vec3 printTone(vec3 c) {
    c = 1.15 * c / (1.0 + 0.15 * c);              // highlight shoulder
    c = mix(c, c * c * (3.0 - 2.0 * c), 0.35);    // S-curve
    c = 0.02 + 0.97 * c;                          // lifted blacks
    float l = luma(c);
    c += (vec3(-0.012, 0.0, 0.018) * (1.0 - l)) + (vec3(0.02, 0.006, -0.02) * l);
    return c;
}

// Dust: a rare speck in a coarse grid of cells, different every frame.
float dust(vec2 px, uint frame, float scale) {
    float cell = 90.0 * scale;
    ivec2 id = ivec2(floor(px / cell));
    uvec3 k = uvec3(uvec2(id), frame + 3u);
    if (rand(k) < 0.9985) return 0.0;
    vec2 center = (vec2(id) + vec2(rand(k + 1u), rand(k + 2u))) * cell;
    float radius = mix(0.8, 2.6, rand(k + 3u)) * scale;
    vec2 d = (px - center) / radius;
    d.x *= mix(1.0, 2.5, rand(k + 4u)); // some specks are fibres
    return smoothstep(1.0, 0.4, length(d));
}

// Display-space highlights that are bright enough to halate.
float highlight(vec3 display) { return smoothstep(0.78, 0.96, luma(display)); }

void main() {
    float film = clamp(u_film, 0.0, 1.0);
    float minRes = min(u_res.x, u_res.y);

    if (u_pass == 0) {
        vec2 uv = (gl_FragCoord.xy - 0.5 * u_res) / minRes;
        vec3 display = pow(fluid(uv, u_time), vec3(1.0 / 2.2));
        fragColor = vec4(display * highlight(display), 1.0);
        return;
    }

    float scale = max(1.0, minRes / 1000.0);   // keep grain size steady on large screens
    uint frame = uint(floor(u_time * 24.0));   // the projector runs at 24 fps

    // Gate weave: the frame jitters a little and drifts slowly in the gate.
    vec2 weave = vec2(rand(uvec3(frame, 1u, 0u)), rand(uvec3(frame, 2u, 0u))) - 0.5;
    weave = (weave * 1.4 + vec2(0.0, 0.8 * sin(u_time * 1.1))) * scale * film;
    vec2 px = gl_FragCoord.xy + weave;
    vec2 uv = (px - 0.5 * u_res) / minRes;

    vec3 clean = fluid(uv, u_time);
    vec3 display = pow(clean, vec3(1.0 / 2.2));

    // Exposure flicker, about two percent.
    float flicker = 1.0 + 0.022 * (rand(uvec3(frame, 5u, 0u)) - 0.5) * 2.0;
    vec3 print = printTone(clean * flicker);
    print = pow(print, vec3(1.0 / 2.2));

    // Halation: bright light passes through the emulsion, reflects off the
    // film base and exposes the red layer again, so highlights grow a warm
    // glow into darker neighbors. The mip chain serves as a wide blur.
    vec2 st = px / u_res;
    vec3 halo = 0.15 * textureLod(u_scene, st, 1.0).rgb
              + 0.30 * textureLod(u_scene, st, 2.5).rgb
              + 0.35 * textureLod(u_scene, st, 3.5).rgb
              + 0.20 * textureLod(u_scene, st, 4.5).rgb;
    float spill = luma(halo) * (1.0 - highlight(print));
    print += spill * vec3(0.95, 0.28, 0.1) * 0.6;

    // Vignette, as from an old lens.
    vec2 v = (gl_FragCoord.xy / u_res - 0.5) * vec2(u_res.x / minRes, u_res.y / minRes);
    print *= mix(1.0, smoothstep(1.3, 0.3, length(v)), 0.2);

    // Grain: two octaves, strongest in the midtones, slightly chromatic.
    vec2 gp = gl_FragCoord.xy / (1.35 * scale);
    float g = 0.75 * grainLayer(gp, frame) + 0.35 * grainLayer(gp * 0.45 + 37.0, frame + 911u);
    float lum = luma(print);
    float amount = 0.016 + 0.03 * (1.0 - pow(abs(2.0 * lum - 1.0), 2.0));
    vec3 tint = vec3(1.0, 0.94, 0.86);
    print += g * amount * tint;

    print = mix(print, print * 0.35, dust(gl_FragCoord.xy, frame, scale));
    print = mix(print, vec3(0.97), 0.8 * dust(gl_FragCoord.xy + 45.0 * scale, frame + 101u, scale));

    fragColor = vec4(clamp(mix(display, print, film), 0.0, 1.0), 1.0);
}
