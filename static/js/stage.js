// Minimal WebGL2 stage: draws one full-screen fragment shader.
// Uniforms every piece may use:
//   u_res  (vec2)  canvas size in device pixels
//   u_time (float) seconds since start
//   u_film (float) 0..1, eased toggle for the analog treatment
"use strict";

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
    gl.viewport(0, 0, canvas.width, canvas.height);
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
    gl.uniform2f(uRes, canvas.width, canvas.height);
    gl.uniform1f(uTime, Number.isFinite(frozen) ? frozen : (now - start) / 1000);
    gl.uniform1f(uFilm, film);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((err) => fail(String(err)));
