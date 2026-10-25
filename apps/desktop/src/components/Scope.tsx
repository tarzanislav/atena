import { useEffect, useState } from "react";

import { api, type Region, type Settings, type Tally } from "../api";
import { useApp } from "../context";
import { languageOf, number } from "../format";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

/** The filters of the chosen data: Slovak data, or foreign data in one language or in all. */
export function scopeFilters(settings: Pick<Settings, "region" | "abroadLanguage">): string[] {
  if (settings.region === "abroad") {
    return settings.abroadLanguage ? ["region/abroad", `language/${settings.abroadLanguage}`] : ["region/abroad"];
  }
  return ["region/sk"];
}

const REGIONS: { region: Region; icon: "location" | "globe" }[] = [
  { region: "sk", icon: "location" },
  { region: "abroad", icon: "globe" },
];

/** Most languages offered as chips (the rest is reached by search). */
const MAX_LANGUAGES = 12;

/**
 * The switch between Slovak and foreign data; foreign data can be narrowed to one language.
 * The choice is a setting, so the overview and search keep it. Counts come from `tally`, or are
 * fetched when none is given.
 */
export function ScopeSwitch({ tally }: { tally?: Tally | null }) {
  const i18n = useI18n();
  const { settings, saveSettings } = useApp();
  const [own, setOwn] = useState<Tally | null>(null);
  const filters = scopeFilters(settings);
  const key = filters.join(" ");
  const fetchOwn = tally === undefined;

  useEffect(() => {
    if (!fetchOwn) return;
    let current = true;
    api.tally(key.split(" ")).then(
      (t) => current && setOwn(t),
      () => current && setOwn(null),
    );
    return () => {
      current = false;
    };
  }, [key, fetchOwn]);

  const counts = tally ?? own;
  const choose = (region: Region, abroadLanguage: string | null) =>
    void saveSettings({ ...settings, region, abroadLanguage });

  const languages = (counts?.languages ?? []).slice(0, MAX_LANGUAGES);
  const chosen = settings.abroadLanguage;
  if (chosen && !languages.some((f) => f.value === chosen)) {
    languages.push({ namespace: "language", value: chosen, count: 0 });
  }

  return (
    <div className="scope">
      <div className="segmented" role="radiogroup" aria-label={i18n.t("scope-label")}>
        {REGIONS.map(({ region, icon }) => (
          <button
            key={region}
            role="radio"
            aria-checked={settings.region === region}
            className="segment"
            onClick={() => choose(region, settings.abroadLanguage)}
          >
            <Icon name={icon} />
            <span>{i18n.t(`scope-${region}`)}</span>
            {counts && <span className="segment-count">{number(i18n, counts.regions[region] ?? 0)}</span>}
          </button>
        ))}
      </div>
      {settings.region === "abroad" && (
        <div className="row" role="group" aria-label={i18n.t("scope-languages")} style={{ gap: 6 }}>
          <button className="chip" aria-pressed={!chosen} onClick={() => choose("abroad", null)}>
            {i18n.t("scope-all-languages")}
          </button>
          {languages.map((f) => (
            <button
              key={f.value}
              className="chip"
              aria-pressed={chosen === f.value}
              onClick={() => choose("abroad", chosen === f.value ? null : f.value)}
            >
              {languageOf(i18n, f.value)}
              <span className="muted">{number(i18n, f.count)}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
