import { describe, expect, it, vi } from "vitest";
import { isHidden, onVisibilityChange, setMinimized } from "./visibility.js";

describe("pencere görünürlüğü", () => {
  it("simge durumu gizli sayılır ve aboneler bilgilendirilir", () => {
    const fn = vi.fn();
    const off = onVisibilityChange(fn);
    expect(isHidden()).toBe(false);
    setMinimized(true);
    expect(isHidden()).toBe(true);
    setMinimized(true); // değişiklik yoksa bildirim yok
    setMinimized(false);
    expect(isHidden()).toBe(false);
    expect(fn).toHaveBeenCalledTimes(2);
    off();
    setMinimized(true);
    expect(fn).toHaveBeenCalledTimes(2);
    setMinimized(false);
  });
});
