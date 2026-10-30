// The page left behind when Search pushes in or pops out
// (crates/ui/src/main.rs `GHOST_JS`): ghost(dir) lays a still copy of the
// page over the main pane as the view changes (under it for a push, over it
// for a pop); ghostGo() sets it moving once the new view lands, and it
// removes itself after.

export function ghost(dir: "push" | "pop", doc: Document = document): void {
  for (const g of doc.querySelectorAll(".view-ghost")) g.remove();
  if (doc.defaultView?.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return;
  const app = doc.querySelector<HTMLElement>(".app");
  const main = app?.querySelector<HTMLElement>(".main");
  const content = main?.querySelector<HTMLElement>(".content");
  if (!app || !main || !content) return;
  const a = app.getBoundingClientRect();
  const m = main.getBoundingClientRect();
  const c = content.getBoundingClientRect();
  const el = doc.createElement("div");
  el.className = `view-ghost ${dir}`;
  const radius = doc.defaultView?.getComputedStyle(main).borderRadius ?? "0";
  el.style.cssText = `left:${m.left - a.left}px;top:${m.top - a.top}px;width:${m.width}px;height:${m.height}px;border-radius:${radius}`;
  const copy = content.cloneNode(true) as HTMLElement;
  copy.classList.remove("view-in-a", "view-in-b", "view-pop");
  copy.style.cssText = `position:absolute;margin:0;top:${c.top - m.top}px;left:${c.left - m.left}px;width:${c.width}px`;
  el.appendChild(copy);
  app.appendChild(el);
  // In case the new view never lands.
  setTimeout(() => el.remove(), 2000);
}

export function ghostGo(doc: Document = document): void {
  for (const g of doc.querySelectorAll(".view-ghost:not(.go)")) {
    g.classList.add("go");
    g.addEventListener("animationend", () => g.remove());
    setTimeout(() => g.remove(), 1500);
  }
}
