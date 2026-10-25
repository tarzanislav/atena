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

/**
 * The sections shown as tiles of the overview: the top-level sections that gather documents.
 * "World and inspiration" is not among them – the switch between Slovak and foreign data does
 * its work.
 */
export const SECTION_TILES = Object.keys(SECTION_COLOURS).filter((id) => id !== "world");

/** Subsections the region switch makes needless (every foreign document would be in them). */
export const HIDDEN_SUBSECTIONS = new Set(["foreign-regulations", "foreign-curricula"]);
