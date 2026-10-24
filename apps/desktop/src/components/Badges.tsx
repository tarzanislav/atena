import { useApp } from "../context";
import { useI18n } from "../i18n";
import { SECTION_COLOURS } from "../sections";

/** The authority of a source (A1–E, named by the taxonomy); A1 and A2 are marked green. */
export function Authority({ value }: { value: string }) {
  const { conceptLabel } = useApp();
  if (!value) return null;
  const high = value === "A1" || value === "A2";
  return (
    <span className={high ? "authority high" : "authority"} title={conceptLabel(`authority/${value}`)}>
      {value}
    </span>
  );
}

/** A taxonomy label as a tag; sections carry their colour. */
export function LabelTag({ label, text }: { label: string; text: string }) {
  const section = label.startsWith("section/") ? label.slice(8) : null;
  const colour = section ? SECTION_COLOURS[section] : undefined;
  return (
    <span className="tag">
      {colour && <span className="tag-dot" style={{ ["--colour" as string]: colour }} />}
      {text}
    </span>
  );
}

/** A yes/no mark with an accessible text. */
export function YesNo({ value, label }: { value: boolean; label: string }) {
  const i18n = useI18n();
  return (
    <span className={value ? "yes" : "no"} title={label}>
      {value ? "✓" : "–"}
      <span className="visually-hidden">{value ? i18n.t("yes") : i18n.t("no")}</span>
    </span>
  );
}
