// The colour of each section of the taxonomy (docs/DESIGN.md) – design, not text.

export const SECTION_COLOURS: Record<string, string> = {
  changes: "#7b8fd6",
  curriculum: "#5fb3a7",
  practice: "#6dbb7f",
  literacies: "#e3a96b",
  inclusion: "#e48aa6",
  digital: "#a48ad6",
  approaches: "#e39a7f",
  "non-formal": "#d4b94e",
  leadership: "#8ea4ba",
  research: "#6aa8d8",
  world: "#9cc06a",
};

/** The sections shown as tiles: the top-level sections that hold documents. */
export const SECTION_TILES = Object.keys(SECTION_COLOURS);

export const NEUTRAL = "#b4b4b4";

/** The colour of a document by the first section among its labels (or the section above it). */
export function colourOf(labels: string[], broaderOf: (key: string) => string | null): string {
  for (const label of labels) {
    let key: string | null = label.startsWith("section/") ? label : null;
    while (key) {
      const colour = SECTION_COLOURS[key.slice("section/".length)];
      if (colour) return colour;
      key = broaderOf(key);
    }
  }
  return NEUTRAL;
}
