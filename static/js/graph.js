// The graph's few interactions: hovering a node lights its neighborhood, a
// click opens it in the inspector, a drag moves it with its links, and the
// filters hide a kind of node. The layout itself comes from the server.
(() => {
  const stage = document.querySelector(".graph-stage");
  const field = stage?.querySelector(".field");
  if (!stage || !field) return;
  const nodes = new Map([...field.querySelectorAll(".node")].map((n) => [n.dataset.id, n]));
  const edges = [...field.querySelectorAll(".edge")];
  const panels = [...document.querySelectorAll(".inspect")];

  const light = (id) => {
    const near = new Set(id ? [id, ...nodes.get(id).dataset.links.split(" ")] : []);
    stage.classList.toggle("lit", Boolean(id));
    nodes.forEach((n, k) => n.classList.toggle("near", near.has(k)));
    edges.forEach((e) => e.classList.toggle("near", Boolean(id) && (e.dataset.a === id || e.dataset.b === id)));
  };
  const focus = (id) => {
    if (!nodes.has(id)) return;
    nodes.forEach((n, k) => n.classList.toggle("focus", k === id));
    panels.forEach((p) => { p.hidden = p.dataset.for !== id; });
  };

  field.addEventListener("pointerover", (ev) => {
    const node = ev.target.closest(".node");
    if (node && !drag) light(node.dataset.id);
  });
  field.addEventListener("pointerout", (ev) => {
    if (!drag && ev.target.closest(".node") && !ev.relatedTarget?.closest?.(".node")) light(null);
  });

  // drag: the node follows the pointer, in percent of the field
  let drag = null;
  field.addEventListener("pointerdown", (ev) => {
    const node = ev.target.closest(".node");
    if (!node || ev.button !== 0) return;
    node.setPointerCapture(ev.pointerId);
    drag = { node, id: node.dataset.id, x: ev.clientX, y: ev.clientY, moved: false };
  });
  field.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    if (!drag.moved && Math.hypot(ev.clientX - drag.x, ev.clientY - drag.y) < 4) return;
    drag.moved = true;
    const box = field.getBoundingClientRect();
    const x = Math.min(100, Math.max(0, ((ev.clientX - box.left) / box.width) * 100));
    const y = Math.min(100, Math.max(0, ((ev.clientY - box.top) / box.height) * 100));
    drag.node.style.left = `${x}%`;
    drag.node.style.top = `${y}%`;
    for (const e of edges) {
      if (e.dataset.a === drag.id) { e.setAttribute("x1", x); e.setAttribute("y1", y); }
      if (e.dataset.b === drag.id) { e.setAttribute("x2", x); e.setAttribute("y2", y); }
    }
  });
  const drop = () => {
    if (drag && !drag.moved) focus(drag.id);
    drag = null;
  };
  field.addEventListener("pointerup", drop);
  field.addEventListener("pointercancel", () => { drag = null; });
  // keyboard: Enter or Space on a node opens it
  field.addEventListener("click", (ev) => {
    const node = ev.target.closest(".node");
    if (node && ev.detail === 0) focus(node.dataset.id);
  });

  document.querySelector(".inspector")?.addEventListener("click", (ev) => {
    const go = ev.target.closest("[data-go]");
    if (!go) return;
    focus(go.dataset.go);
    nodes.get(go.dataset.go)?.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  });
  document.querySelector(".inspector")?.addEventListener("pointerover", (ev) => {
    const go = ev.target.closest("[data-go]");
    light(go ? go.dataset.go : null);
  });
  document.querySelector(".inspector")?.addEventListener("pointerleave", () => light(null));

  for (const button of document.querySelectorAll(".filters [data-kind]")) {
    button.addEventListener("click", () => {
      const on = button.getAttribute("aria-pressed") !== "true";
      button.setAttribute("aria-pressed", String(on));
      button.classList.toggle("active", on);
      stage.classList.toggle(`hide-${button.dataset.kind}`, !on);
    });
  }
})();
