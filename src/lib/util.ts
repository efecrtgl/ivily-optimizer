// Küçük yardımcılar.

export function sortBy<T>(arr: T[], sort: { key: string; dir: 1 | -1 }): T[] {
  const k = sort.key as keyof T;
  return [...arr].sort((a, b) => {
    const va = a[k] as any;
    const vb = b[k] as any;
    if (va == null && vb == null) return 0;
    if (va == null) return 1;
    if (vb == null) return -1;
    if (typeof va === "string") return va.localeCompare(vb, "tr") * sort.dir;
    return (va > vb ? 1 : va < vb ? -1 : 0) * sort.dir;
  });
}

export function matches(text: string | null | undefined, q: string): boolean {
  if (!q) return true;
  return (text ?? "").toLocaleLowerCase("tr").includes(q.toLocaleLowerCase("tr"));
}

export function clamp(v: number, a: number, b: number) {
  return Math.min(b, Math.max(a, v));
}
