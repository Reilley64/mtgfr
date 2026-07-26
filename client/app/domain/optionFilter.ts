/** Case-insensitive substring filter for closed option lists (creature types, etc.). */
export function filterOptionLabels(options: ReadonlyArray<string>, query: string): string[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...options];
  return options.filter((option) => option.toLowerCase().includes(q));
}
