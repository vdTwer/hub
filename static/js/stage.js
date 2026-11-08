// Minimal WebGL2 stage: draws one full-screen fragment shader.
// Uniforms every piece may use:
//   u_res  (vec2)  canvas size in device pixels
//   u_time (float) seconds since start
//   u_film (float) 0..1, eased toggle for the analog treatment
//   u_stir (vec4[12]) recent pointer samples: position xy, velocity zw,
//                     in units of the short side, origin at the center
//   u_stirAge (float[12]) seconds since each sample, -1 when unused
// A piece that declares `sampler2D u_scene` gets two passes per frame:
// u_pass 0 renders at quarter size into u_scene, which is then mipmapped
// (a cheap wide blur), and u_pass 1 renders the frame to the screen.
"use strict";

const STIR = 12;
const SCENE_DIVISOR = 4;

const VERT = `#version 300 es
in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }`;

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) ?? "shader compile failed");
  }
  return shader;
}

function link(gl, fragSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragSource));
  gl.bindAttribLocation(program, 0, "a_pos");
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) ?? "program link failed");
  }
  return program;
}

function fail(message) {
  console.error(message);
  document.body.dataset.error = "true";
  const note = document.createElement("pre");
  note.className = "error";
  note.textContent = message;
  document.body.append(note);
}

async function main() {
  const canvas = document.getElementById("stage");
  const gl = canvas.getContext("webgl2", { antialias: false, alpha: false });
  if (!gl) return fail("WebGL2 is not available in this browser.");

  const response = await fetch(canvas.dataset.shader);
  if (!response.ok) return fail(`Could not load ${canvas.dataset.shader}`);
  const program = link(gl, await response.text());
  gl.useProgram(program);

  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

  const u = (name) => gl.getUniformLocation(program, name);
  const uRes = u("u_res"), uTime = u("u_time"), uFilm = u("u_film");
  const uPass = u("u_pass"), uScene = u("u_scene");
  const uStir = u("u_stir"), uStirAge = u("u_stirAge");

  // Pointer trail, newest first. Positions use the shader's uv convention.
  const trail = [];
  let lastMove = null;
  canvas.addEventListener("pointermove", (e) => {
    const w = canvas.clientWidth, h = canvas.clientHeight, s = Math.min(w, h);
    const pos = [(e.clientX - w / 2) / s, (h / 2 - e.clientY) / s];
    const now = performance.now();
    if (lastMove && now - lastMove.t < 16) return;
    const dt = lastMove ? Math.max((now - lastMove.t) / 1000, 1e-3) : 1;
    const vel = lastMove && dt < 0.5
      ? [(pos[0] - lastMove.pos[0]) / dt, (pos[1] - lastMove.pos[1]) / dt]
      : [0, 0];
    lastMove = { pos, t: now };
    trail.unshift({ pos, vel, t: now });
    trail.length = Math.min(trail.length, STIR);
  });
  const stirData = new Float32Array(4 * STIR), stirAge = new Float32Array(STIR);
  const uploadTrail = (now) => {
    for (let i = 0; i < STIR; i++) {
      const sample = trail[i];
      stirAge[i] = sample ? (now - sample.t) / 1000 : -1;
      if (sample) stirData.set([...sample.pos, ...sample.vel], 4 * i);
    }
    gl.uniform4fv(uStir, stirData);
    gl.uniform1fv(uStirAge, stirAge);
  };

  // Optional scene pre-pass into a mipmapped texture.
  let scene = null;
  const makeScene = () => {
    if (!uScene) return;
    if (scene) { gl.deleteTexture(scene.tex); gl.deleteFramebuffer(scene.fbo); }
    const w = Math.max(1, Math.round(canvas.width / SCENE_DIVISOR));
    const h = Math.max(1, Math.round(canvas.height / SCENE_DIVISOR));
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    const levels = Math.floor(Math.log2(Math.max(w, h))) + 1;
    gl.texStorage2D(gl.TEXTURE_2D, levels, gl.RGBA8, w, h);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    scene = { tex, fbo, w, h };
  };

  // Film toggle, eased so the switch reads as a dissolve.
  const params = new URLSearchParams(location.search);
  const button = document.getElementById("film");
  let filmTarget = params.get("film") === "0" ? 0 : 1, film = filmTarget;
  const toggle = () => {
    filmTarget = 1 - filmTarget;
    button.setAttribute("aria-pressed", String(filmTarget === 1));
    button.textContent = `Film: ${filmTarget ? "on" : "off"}`;
  };
  button?.addEventListener("click", toggle);
  if (filmTarget === 0) { filmTarget = 1; toggle(); }
  addEventListener("keydown", (e) => { if (e.key === "f") toggle(); });

  const resize = () => {
    const dpr = Math.min(devicePixelRatio || 1, 2);
    canvas.width = Math.round(canvas.clientWidth * dpr);
    canvas.height = Math.round(canvas.clientHeight * dpr);
    makeScene();
  };
  addEventListener("resize", resize);
  resize();

  // ?t=12.5 freezes time and ?film=0 starts clean: handy for stills.
  const frozen = parseFloat(params.get("t"));
  const start = performance.now();
  let last = start;
  const frame = (now) => {
    film += (filmTarget - film) * (1 - Math.exp(-(now - last) / 180));
    last = now;
    gl.uniform1f(uTime, Number.isFinite(frozen) ? frozen : (now - start) / 1000);
    gl.uniform1f(uFilm, film);
    uploadTrail(now);

    if (scene) {
      gl.bindTexture(gl.TEXTURE_2D, null); // never sample what we draw into
      gl.bindFramebuffer(gl.FRAMEBUFFER, scene.fbo);
      gl.viewport(0, 0, scene.w, scene.h);
      gl.uniform2f(uRes, scene.w, scene.h);
      gl.uniform1i(uPass, 0);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, scene.tex);
      gl.generateMipmap(gl.TEXTURE_2D);
      gl.uniform1i(uScene, 0);
    }
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.uniform2f(uRes, canvas.width, canvas.height);
    gl.uniform1i(uPass, 1);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((err) => fail(String(err)));
