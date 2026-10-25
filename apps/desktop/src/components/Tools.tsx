import { useEffect, useState } from "react";

import { api, type Settings, type Tool } from "../api";
import { useApp } from "../context";
import { languageOf } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { Trust } from "./Badges";

/** The tools of the register, described in the interface language. */
export function useTools(): Tool[] {
  const { settings } = useApp();
  const [tools, setTools] = useState<Tool[]>([]);
  useEffect(() => {
    api.tools(settings.language).then(setTools, () => setTools([]));
  }, [settings.language]);
  return tools;
}

/** The tools of the chosen data: Slovak, or foreign in the chosen language. */
export function inScope(tools: Tool[], settings: Pick<Settings, "region" | "abroadLanguage">): Tool[] {
  return tools.filter(
    (t) =>
      t.region === settings.region &&
      (settings.region === "sk" || !settings.abroadLanguage || t.languages.includes(settings.abroadLanguage)),
  );
}

/** Text without case and diacritics, for matching words. */
function folded(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Tools whose name or description contains a word of `text` (of three letters or more). */
export function matching(tools: Tool[], text: string): Tool[] {
  const words = folded(text)
    .split(/[^\p{L}\p{N}]+/u)
    .filter((w) => w.length >= 3);
  if (words.length === 0) return [];
  return tools.filter((t) => {
    const hay = folded(`${t.name} ${t.description}`);
    return words.some((w) => hay.includes(w));
  });
}

/** Tools as a list: name, description and a button to open them in the browser. */
export function ToolList({ tools, title }: { tools: Tool[]; title: string }) {
  const i18n = useI18n();
  if (tools.length === 0) return null;
  return (
    <section className="card tight" aria-label={title}>
      <div className="item-body" style={{ padding: "8px 14px 6px" }}>
        <h2>{title}</h2>
        <span className="small muted">{i18n.t("tools-intro")}</span>
      </div>
      {tools.map((t) => (
        <div key={t.id} className="list-item">
          <span className={`bar trust-bar ${t.trust}`} />
          <div className="item-body">
            <strong className="item-title">{t.name}</strong>
            <div className="meta">
              <Trust value={t.trust} authority="" />
              <span>{t.languages.map((l) => languageOf(i18n, l)).join(", ")}</span>
            </div>
            <span className="small">{t.description}</span>
          </div>
          <div>
            <button className="button" onClick={() => void api.openLink(t.url)} title={t.url}>
              <Icon name="open" size={16} />
              {i18n.t("tools-open")}
            </button>
          </div>
        </div>
      ))}
    </section>
  );
}
