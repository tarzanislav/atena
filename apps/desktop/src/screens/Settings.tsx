import { useState } from "react";

import { api, type Erasure, type Theme } from "../api";
import { ErrorNotice } from "../components/ErrorNotice";
import { useApp } from "../context";
import { LANGUAGES, languageName, useI18n } from "../i18n";
import { Icon } from "../icons";

export function SettingsScreen() {
  const i18n = useI18n();
  const { settings, saveSettings, version } = useApp();
  const [editor, setEditor] = useState(settings.editor);
  const [error, setError] = useState<unknown>(null);
  const [saved, setSaved] = useState(false);

  const save = async (change: Partial<typeof settings>) => {
    setError(null);
    setSaved(false);
    try {
      await saveSettings({ ...settings, ...change });
      setSaved(true);
    } catch (e) {
      setError(e);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <div className="row spread">
          <h1>{i18n.t("settings-title")}</h1>
          {/* a quiet confirmation that does not move the page */}
          <span className="row small" role="status" style={{ visibility: saved ? "visible" : "hidden", color: "var(--green-ink)" }}>
            <Icon name="checkmark" size={16} />
            {i18n.t("settings-saved")}
          </span>
        </div>
      </div>
      <ErrorNotice error={error} />

      <div className="grid two">
        <div className="card">
          <h2>{i18n.t("settings-appearance")}</h2>
          <label className="field">
            <span>{i18n.t("settings-language")}</span>
            <select className="select" value={settings.language} onChange={(e) => void save({ language: e.target.value })}>
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {languageName(l.code)}
                  {l.draft ? ` – ${i18n.t("translation-draft")}` : ""}
                </option>
              ))}
            </select>
          </label>
          <fieldset className="field" style={{ border: 0, padding: 0, margin: 0 }}>
            <span>{i18n.t("settings-theme")}</span>
            <div className="row">
              {(["system", "light", "dark"] as Theme[]).map((theme) => (
                <button key={theme} className="chip" aria-pressed={settings.theme === theme} onClick={() => void save({ theme })}>
                  <Icon name={theme === "dark" ? "moon" : theme === "light" ? "sun" : "settings"} size={16} />
                  {i18n.t(`settings-theme-${theme}`)}
                </button>
              ))}
            </div>
          </fieldset>
        </div>

        <div className="card">
          <h2>{i18n.t("settings-decisions")}</h2>
          <label className="field">
            <span>{i18n.t("settings-editor")}</span>
            <input
              className="input"
              value={editor}
              maxLength={80}
              onChange={(e) => setEditor(e.target.value)}
              onBlur={() => editor !== settings.editor && void save({ editor })}
            />
          </label>
          <p className="small muted">{i18n.t("settings-editor-privacy")}</p>
          <label className="check">
            <input type="checkbox" checked={settings.shareProposals} onChange={(e) => void save({ shareProposals: e.target.checked })} />
            <span>
              {i18n.t("settings-share")}
              <br />
              <span className="small muted">{i18n.t("settings-share-explain")}</span>
            </span>
          </label>
        </div>

        <div className="card">
          <h2>{i18n.t("settings-crawler")}</h2>
          <label className="field">
            <span>{i18n.t("settings-concurrency")}</span>
            <input
              className="input"
              type="number"
              min={1}
              max={16}
              value={settings.concurrency}
              onChange={(e) => void save({ concurrency: Number(e.target.value) || 1 })}
              style={{ maxWidth: 120 }}
            />
          </label>
          <p className="small muted">{i18n.t("settings-concurrency-explain")}</p>
        </div>

        <ErasureCard />
      </div>

      <div className="card lavender">
        <h2>{i18n.t("about-title")}</h2>
        <p>{i18n.t("about-version", { version })}</p>
        <p>{i18n.t("about-connections")}</p>
        <h3>{i18n.t("about-pillars")}</h3>
        <ol style={{ margin: 0, paddingLeft: 22, display: "flex", flexDirection: "column", gap: 4 }}>
          {[1, 2, 3, 4].map((n) => (
            <li key={n}>{i18n.t(`pillar-${n}`)}</li>
          ))}
        </ol>
        <p className="small muted">{i18n.t("about-licence")}</p>
        <div>
          <button className="button" onClick={() => void api.openLink("https://github.com/tarzanislav/atena")}>
            <Icon name="open" size={16} />
            {i18n.t("about-repository")}
          </button>
        </div>
      </div>
    </div>
  );
}

function ErasureCard() {
  const i18n = useI18n();
  const [url, setUrl] = useState("");
  const [prefix, setPrefix] = useState(false);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [done, setDone] = useState<Erasure | null>(null);

  const erase = async () => {
    setBusy(true);
    setError(null);
    setDone(null);
    try {
      setDone(await api.forget(url, prefix, reason));
      setUrl("");
      setReason("");
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="card">
      <h2>{i18n.t("erasure-title")}</h2>
      <p className="small muted">{i18n.t("erasure-explain")}</p>
      <label className="field">
        <span>{i18n.t("erasure-address")}</span>
        <input className="input" value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://" spellCheck={false} />
      </label>
      <label className="check">
        <input type="checkbox" checked={prefix} onChange={(e) => setPrefix(e.target.checked)} />
        <span>{i18n.t("erasure-prefix")}</span>
      </label>
      <label className="field">
        <span>{i18n.t("erasure-reason")}</span>
        <input className="input" value={reason} maxLength={2000} onChange={(e) => setReason(e.target.value)} />
      </label>
      <ErrorNotice error={error} />
      {done && (
        <div className="notice ok" role="status">
          <Icon name="checkmark" />
          <span>{i18n.t("erasure-done", { documents: done.documents, versions: done.versions })}</span>
        </div>
      )}
      <div>
        <button className="button danger" disabled={busy || !url.trim() || !reason.trim()} onClick={() => void erase()}>
          <Icon name="erase" size={16} />
          {i18n.t("erasure-action")}
        </button>
      </div>
    </div>
  );
}
