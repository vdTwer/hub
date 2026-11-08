// Opens the note a link points at (#n3) without a round trip: the list and
// the reader follow the hash. Without it, the server's open note stays.
(() => {
  const open = () => {
    const view = document.getElementById(location.hash.slice(1));
    if (!view?.classList.contains("note-view")) return;
    for (const v of document.querySelectorAll(".note-view")) v.classList.toggle("open", v === view);
    for (const r of document.querySelectorAll(".note-row")) {
      const on = r.dataset.note === view.id;
      r.classList.toggle("open", on);
      r.querySelector("a")?.toggleAttribute("aria-current", on);
    }
    document.querySelector(".reader")?.scrollTo(0, 0);
  };
  addEventListener("hashchange", open);
  open();
})();
