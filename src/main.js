import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// Sağ tık menüsünü (tarayıcı menüsü) kapat; metin kutularında açık kalsın.
window.addEventListener("contextmenu", (e) => {
  const t = e.target;
  if (!t.closest("input, textarea, [contenteditable]")) e.preventDefault();
});

const app = mount(App, { target: document.getElementById("app") });
export default app;
