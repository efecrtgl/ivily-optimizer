// Küçük yardımcılar.

export function sortBy(arr, sort) {
  const k = sort.key;
  return [...arr].sort((a, b) => {
    const va = a[k] ;
    const vb = b[k] ;
    if (va == null && vb == null) return 0;
    if (va == null) return 1;
    if (vb == null) return -1;
    if (typeof va === "string") return va.localeCompare(vb, "tr") * sort.dir;
    return (va > vb ? 1 : va < vb ? -1 : 0) * sort.dir;
  });
}

export function matches(text, q) {
  if (!q) return true;
  return (text ?? "").toLocaleLowerCase("tr").includes(q.toLocaleLowerCase("tr"));
}

export function clamp(v, a, b) {
  return Math.min(b, Math.max(a, v));
}
