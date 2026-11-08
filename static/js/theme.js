// Darkroom: picks the theme before the page paints, from ?theme=, then the
// viewer's last choice, then the system. Any .theme-toggle button and the D
// key switch it, and the choice is remembered where storage allows.
"use strict";
(() => {
  const root = document.documentElement;
  const read = () => { try { return localStorage.getItem("theme"); } catch { return null; } };
  const save = (t) => { try { localStorage.setItem("theme", t); } catch { /* private mode */ } };
  const asked = new URLSearchParams(location.search).get("theme");
  const system = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  root.dataset.theme = ["dark", "light"].includes(asked) ? asked : read() ?? system;

  const sync = () => document.querySelectorAll(".theme-toggle").forEach((b) => {
    b.setAttribute("aria-pressed", String(root.dataset.theme === "dark"));
  });
  const toggle = () => {
    root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
    save(root.dataset.theme);
    sync();
  };
  addEventListener("DOMContentLoaded", () => {
    sync();
    document.querySelectorAll(".theme-toggle").forEach((b) => b.addEventListener("click", toggle));
  });
  addEventListener("keydown", (e) => {
    if (e.key === "d" && !(e.target instanceof HTMLInputElement)) toggle();
  });
})();
