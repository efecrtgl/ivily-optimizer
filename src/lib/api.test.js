// Sözleşme testi: arayüzün çağırdığı her Rust komutu, arka uçta gerçekten
// kayıtlı olmalıdır. TypeScript kaldırıldıktan sonra arayüz ile arka uç
// arasındaki adlandırma kaymasını yakalayan ana güvencedir.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { beforeEach, describe, expect, it, vi } from "vitest";

const calls = [];
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((cmd, args) => {
    calls.push({ cmd, args });
    return Promise.resolve(null);
  }),
}));

const root = join(import.meta.dirname, "..", "..");

/** `generate_handler![...]` içindeki komut adları. */
function registeredCommands() {
  const lib = readFileSync(join(root, "src-tauri", "src", "lib.rs"), "utf8");
  const block = lib.match(/generate_handler!\[([\s\S]*?)\]/)[1];
  return new Set([...block.matchAll(/::(\w+)\s*,/g)].map((m) => m[1]));
}

/** Arayüz kaynaklarında doğrudan `invoke("...")` ile çağrılan komutlar. */
function directInvokes() {
  const out = new Set();
  const walk = (dir) => {
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = join(dir, e.name);
      if (e.isDirectory()) walk(p);
      else if (/\.(js|svelte)$/.test(e.name) && !e.name.endsWith(".test.js")) {
        for (const m of readFileSync(p, "utf8").matchAll(/\binvoke\(\s*"(\w+)"/g)) out.add(m[1]);
      }
    }
  };
  walk(join(root, "src"));
  return out;
}

describe("Rust komut sözleşmesi", () => {
  beforeEach(() => {
    calls.length = 0;
  });

  it("api.js içindeki her yöntem kayıtlı bir komutu çağırır", async () => {
    const mod = await import("./api.js");
    const registered = registeredCommands();
    let methods = 0;
    for (const [group, obj] of Object.entries(mod)) {
      for (const [name, fn] of Object.entries(obj)) {
        if (typeof fn !== "function") continue;
        calls.length = 0;
        await fn("x", "y", "z", "w");
        methods++;
        expect(calls.length, `${group}.${name} invoke çağırmadı`).toBeGreaterThan(0);
        for (const c of calls) expect(registered.has(c.cmd), `${group}.${name} → ${c.cmd} kayıtlı değil`).toBe(true);
      }
    }
    expect(methods).toBeGreaterThan(50);
  });

  it("doğrudan invoke edilen komutlar da kayıtlıdır", () => {
    const registered = registeredCommands();
    const direct = directInvokes();
    expect(direct.size).toBeGreaterThan(0);
    for (const cmd of direct) expect(registered.has(cmd), `${cmd} kayıtlı değil`).toBe(true);
  });
});
