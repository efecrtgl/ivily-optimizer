import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { initWindowVisibility } from "./lib/visibility.js";

// Sağ tık menüsünü (tarayıcı menüsü) kapat; metin kutularında açık kalsın.
window.addEventListener("contextmenu", (e) => {
  const t = e.target;
  if (!t.closest("input, textarea, [contenteditable]")) e.preventDefault();
});

// Pencerenin görünürlüğü (simge durumu dahil) izlenir: gizliyken CSS
// animasyonları ve canlı yoklama durur.
initWindowVisibility();

const app = mount(App, { target: document.getElementById("app") });
export default app;
