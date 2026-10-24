import { useCallback, useEffect, useState } from "react";

import {
  api,
  type Allowed,
  type DecisionOptions,
  type ImportResult,
  type Pending,
  type Proposal,
  type Proposed,
  type Received,
  type Rejected,
} from "../api";
import { YesNo } from "../components/Badges";
import { Dialog } from "../components/Dialog";
import { ErrorNotice } from "../components/ErrorNotice";
import { useApp } from "../context";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

type Tab = "add" | "waiting" | "decided" | "share" | "receive";

export function ProposalsScreen() {
  const i18n = useI18n();
  const [tab, setTab] = useState<Tab>("add");
  const tabs: Tab[] = ["add", "waiting", "decided", "share", "receive"];
  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("proposals-title")}</h1>
        <p className="muted">{i18n.t("proposals-intro")}</p>
      </div>
      <div className="tabs" role="tablist">
        {tabs.map((t) => (
          <button key={t} className="tab" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
            {i18n.t(`proposals-tab-${t}`)}
          </button>
        ))}
      </div>
      {tab === "add" && <AddPanel />}
      {tab === "waiting" && <ProposalList state="proposed" />}
      {tab === "decided" && <ProposalList state="all" />}
      {tab === "share" && <SharePanel />}
      {tab === "receive" && <ReceivePanel />}
    </div>
  );
}

function useOptions() {
  const [options, setOptions] = useState<DecisionOptions | null>(null);
  useEffect(() => {
    api.decisionOptions().then(setOptions, () => undefined);
  }, []);
  return options;
}

function CountrySelect({
  value,
  onChange,
  countries,
  empty,
}: {
  value: string;
  onChange: (v: string) => void;
  countries: string[];
  empty?: string;
}) {
  const { conceptLabel } = useApp();
  return (
    <select className="select" value={value} onChange={(e) => onChange(e.target.value)}>
      {empty !== undefined && <option value="">{empty}</option>}
      {countries.map((c) => (
        <option key={c} value={c}>
          {conceptLabel(`country/${c}`)} ({c})
        </option>
      ))}
    </select>
  );
}

function AllowedLine({ allowed }: { allowed: Allowed }) {
  const i18n = useI18n();
  return (
    <span className="row small" style={{ gap: 14 }}>
      <span>
        <YesNo value={allowed.crawl} label={i18n.t("allowed-crawl")} /> {i18n.t("allowed-crawl")}
      </span>
      <span>
        <YesNo value={allowed.storeText} label={i18n.t("allowed-text")} /> {i18n.t("allowed-text")}
      </span>
      <span>
        <YesNo value={allowed.showFullText} label={i18n.t("allowed-show")} /> {i18n.t("allowed-show")}
      </span>
    </span>
  );
}

function ReasonTags({ reasons }: { reasons: string[] }) {
  const i18n = useI18n();
  if (reasons.length === 0) return null;
  return (
    <span className="row" style={{ gap: 6 }}>
      {reasons.map((r) => (
        <span key={r} className="tag">
          {i18n.label("reason", r)}
        </span>
      ))}
    </span>
  );
}

function ProposedTable({ proposed, rejected }: { proposed: Proposed[]; rejected: Rejected[] }) {
  const i18n = useI18n();
  return (
    <div className="card" style={{ padding: "8px 10px", overflowX: "auto" }}>
      <table className="table">
        <thead>
          <tr>
            <th>{i18n.t("import-column-line")}</th>
            <th>{i18n.t("import-column-address")}</th>
            <th>{i18n.t("import-column-result")}</th>
            <th>{i18n.t("import-column-proposal")}</th>
          </tr>
        </thead>
        <tbody>
          {proposed.map((p) => (
            <tr key={`${p.line}-${p.url}`}>
              <td>{p.line}</td>
              <td className="small" style={{ overflowWrap: "anywhere" }}>
                {p.title && <div>{p.title}</div>}
                <div className="muted">{p.url}</div>
              </td>
              <td>
                {i18n.label("outcome", p.outcome, { register: p.registerId ?? "" })}
                {p.problem && <div className="small muted">{i18n.label("problem", p.problem)}</div>}
              </td>
              <td className="small">
                {p.outcome !== "personal-page" && (
                  <>
                    <div>
                      {i18n.label("licence", p.licence)} · {i18n.label("terms", p.terms)}
                      {p.country ? ` · ${p.country}` : ""}
                    </div>
                    <ReasonTags reasons={p.reasons} />
                  </>
                )}
              </td>
            </tr>
          ))}
          {rejected.map((r) => (
            <tr key={`r-${r.line}`}>
              <td>{r.line}</td>
              <td className="small muted" style={{ overflowWrap: "anywhere" }}>
                {r.text}
              </td>
              <td colSpan={2}>{i18n.label("rejection", r.reason)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function AddPanel() {
  const i18n = useI18n();
  const { refreshBadges } = useApp();
  const options = useOptions();
  const [text, setText] = useState("");
  const [country, setCountry] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const lines = text.split("\n").filter((l) => l.trim()).length;

  const assess = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      setResult(await api.importAddresses(text, country || null));
      refreshBadges();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="item-body" style={{ gap: 16 }}>
      <div className="card">
        <label className="field">
          <span>{i18n.t("import-addresses")}</span>
          <textarea
            className="textarea"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder={i18n.t("import-placeholder")}
            spellCheck={false}
          />
        </label>
        <p className="small muted">{i18n.t("import-format")}</p>
        <div className="row">
          <label className="field">
            <span>{i18n.t("import-country")}</span>
            {options && (
              <CountrySelect
                value={country}
                onChange={setCountry}
                countries={options.countries}
                empty={i18n.t("import-country-auto")}
              />
            )}
          </label>
        </div>
        <div className="notice info">
          <Icon name="shield" />
          <span>{i18n.t("import-how")}</span>
        </div>
        <div className="row">
          <button className="button primary" disabled={busy || lines === 0} onClick={() => void assess()}>
            {busy ? <div className="spinner" /> : <Icon name="add" size={16} />}
            {i18n.t("import-action", { count: lines })}
          </button>
          {busy && <span className="muted small">{i18n.t("import-busy")}</span>}
        </div>
      </div>
      <ErrorNotice error={error} />
      {result && (
        <>
          <div className="notice ok" role="status">
            <Icon name="checkmark" />
            <span>
              {i18n.t("import-done", { count: result.proposed.length })}{" "}
              {result.keptForSharing > 0 && i18n.t("import-kept-for-sharing", { count: result.keptForSharing })}
            </span>
          </div>
          <ProposedTable proposed={result.proposed} rejected={result.rejected} />
        </>
      )}
    </div>
  );
}

function ProposalList({ state }: { state: "proposed" | "all" }) {
  const i18n = useI18n();
  const { refreshBadges } = useApp();
  const [items, setItems] = useState<Proposal[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [deciding, setDeciding] = useState<{ proposal: Proposal; confirm: boolean } | null>(null);

  const load = useCallback(
    () =>
      api.proposals(state).then(
        (all) => setItems(state === "all" ? all.filter((p) => p.state !== "proposed") : all),
        setError,
      ),
    [state],
  );
  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="item-body" style={{ gap: 14 }}>
      <ErrorNotice error={error} />
      {items === null && !error && <div className="spinner" />}
      {items?.length === 0 && (
        <div className="card empty">{i18n.t(state === "proposed" ? "proposals-none-waiting" : "proposals-none-decided")}</div>
      )}
      {items?.map((p) => (
        <div key={p.id} className="card">
          <div className="row spread" style={{ alignItems: "flex-start" }}>
            <div className="item-body">
              <button className="title-button item-title" onClick={() => void api.openLink(p.url)} title={p.url}>
                {p.name}
              </button>
              <span className="meta">
                <span>{p.id}</span>
                <span className="tag">{i18n.label("state", p.state)}</span>
                {p.shared && <span className="tag">{i18n.t("proposal-shared")}</span>}
                <span style={{ overflowWrap: "anywhere" }}>{p.url}</span>
              </span>
            </div>
            {p.state === "proposed" && (
              <div className="row">
                <button className="button primary" onClick={() => setDeciding({ proposal: p, confirm: true })}>
                  <Icon name="checkmark" size={16} />
                  {i18n.t("proposal-confirm")}
                </button>
                <button className="button danger" onClick={() => setDeciding({ proposal: p, confirm: false })}>
                  <Icon name="dismiss" size={16} />
                  {i18n.t("proposal-reject")}
                </button>
              </div>
            )}
          </div>
          {p.state !== "proposed" && (
            <p className="small">
              {i18n.t("proposal-decided", {
                licence: i18n.label("licence", p.licence),
                terms: i18n.label("terms", p.terms),
                country: p.country || "?",
              })}
            </p>
          )}
          {p.proposal && (
            <div className="item-body" style={{ gap: 8 }}>
              <p className="small">
                {i18n.t("proposal-automatic", {
                  licence: i18n.label("licence", p.proposal.licence),
                  terms: i18n.label("terms", p.proposal.terms),
                  country: p.proposal.country ?? "?",
                  date: p.proposal.checked,
                })}
              </p>
              {p.proposal.problem && <p className="small muted">{i18n.label("problem", p.proposal.problem)}</p>}
              <ReasonTags reasons={p.proposal.reasons} />
              {p.state === "proposed" && (
                <div className="item-body" style={{ gap: 4 }}>
                  <span className="small muted">{i18n.t("proposal-would-allow")}</span>
                  <AllowedLine allowed={p.proposal.allowed} />
                </div>
              )}
              <details>
                <summary>{i18n.t("proposal-finding")}</summary>
                <p className="detail">{p.proposal.finding}</p>
                {p.proposal.rightsPage && (
                  <button className="button link" onClick={() => void api.openLink(p.proposal!.rightsPage)}>
                    <Icon name="open" size={16} />
                    {i18n.t("proposal-rights-page")}
                  </button>
                )}
              </details>
            </div>
          )}
        </div>
      ))}
      {deciding && (
        <DecisionDialog
          proposal={deciding.proposal}
          confirm={deciding.confirm}
          onClose={() => setDeciding(null)}
          onDone={() => {
            setDeciding(null);
            refreshBadges();
            void load();
          }}
        />
      )}
    </div>
  );
}

function DecisionDialog({
  proposal,
  confirm,
  onClose,
  onDone,
}: {
  proposal: Proposal;
  confirm: boolean;
  onClose: () => void;
  onDone: () => void;
}) {
  const i18n = useI18n();
  const { settings, navigate } = useApp();
  const options = useOptions();
  const [licence, setLicence] = useState(proposal.proposal?.licence ?? proposal.licence);
  const [terms, setTerms] = useState(proposal.proposal?.terms ?? proposal.terms);
  const [country, setCountry] = useState(proposal.proposal?.country ?? proposal.country ?? "");
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [done, setDone] = useState<string | null>(null);

  const decide = async () => {
    setBusy(true);
    setError(null);
    try {
      if (confirm) {
        const c = await api.confirm(proposal.id, { licence, terms, country: country || null }, reason);
        setDone(i18n.t(c.crawl ? "proposal-confirmed-crawled" : "proposal-confirmed-not-crawled"));
      } else {
        await api.reject(proposal.id, reason);
        setDone(i18n.t("proposal-rejected"));
      }
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog title={i18n.t(confirm ? "proposal-confirm-title" : "proposal-reject-title", { name: proposal.name })} onClose={done ? onDone : onClose} narrow>
      {!settings.editor && (
        <div className="notice error">
          <Icon name="warning" />
          <div className="item-body">
            <span>{i18n.t("error-editor-missing")}</span>
            <button className="button link" style={{ alignSelf: "flex-start" }} onClick={() => navigate("settings")}>
              {i18n.t("nav-settings")}
            </button>
          </div>
        </div>
      )}
      {done ? (
        <>
          <div className="notice ok" role="status">
            <Icon name="checkmark" />
            <span>{done}</span>
          </div>
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="button primary" onClick={onDone}>
              {i18n.t("action-close")}
            </button>
          </div>
        </>
      ) : (
        <>
          {confirm && options && (
            <div className="form-grid">
              <label className="field">
                <span>{i18n.t("decision-licence")}</span>
                <select className="select" value={licence} onChange={(e) => setLicence(e.target.value)}>
                  {options.licences.map((l) => (
                    <option key={l} value={l}>
                      {i18n.label("licence", l)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>{i18n.t("decision-terms")}</span>
                <select className="select" value={terms} onChange={(e) => setTerms(e.target.value)}>
                  {options.terms.map((t) => (
                    <option key={t} value={t}>
                      {i18n.label("terms", t)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>{i18n.t("decision-country")}</span>
                <CountrySelect value={country} onChange={setCountry} countries={options.countries} empty="–" />
              </label>
            </div>
          )}
          <label className="field">
            <span>{i18n.t(confirm ? "decision-reason-confirm" : "decision-reason-reject")}</span>
            <textarea className="textarea prose" value={reason} maxLength={2000} onChange={(e) => setReason(e.target.value)} />
          </label>
          <p className="small muted">{i18n.t("decision-by", { name: settings.editor || "–" })}</p>
          <ErrorNotice error={error} />
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button className="button" onClick={onClose}>
              {i18n.t("action-cancel")}
            </button>
            <button
              className={confirm ? "button primary" : "button primary danger"}
              disabled={busy || !reason.trim() || !settings.editor}
              onClick={() => void decide()}
            >
              {busy && <div className="spinner" />}
              {i18n.t(confirm ? "proposal-confirm" : "proposal-reject")}
            </button>
          </div>
        </>
      )}
    </Dialog>
  );
}

function SharePanel() {
  const i18n = useI18n();
  const [pending, setPending] = useState<Pending | null | undefined>(undefined);
  const [error, setError] = useState<unknown>(null);
  const [copied, setCopied] = useState(false);
  const [handed, setHanded] = useState(false);

  const load = () => api.pendingShare().then(setPending, setError);
  useEffect(() => {
    void load();
  }, []);

  const copy = async () => {
    if (!pending) return;
    try {
      await navigator.clipboard.writeText(pending.list);
      setCopied(true);
    } catch (e) {
      setError(e);
    }
  };
  const handOver = async () => {
    if (!pending) return;
    try {
      await api.handedOver(pending.entries);
      setHanded(true);
      setCopied(false);
      await load();
    } catch (e) {
      setError(e);
    }
  };

  return (
    <div className="item-body" style={{ gap: 14 }}>
      <div className="notice info">
        <Icon name="shield" />
        <span>{i18n.t("share-explain")}</span>
      </div>
      <ErrorNotice error={error} />
      {handed && (
        <div className="notice ok" role="status">
          <Icon name="checkmark" />
          <span>{i18n.t("share-handed-over")}</span>
        </div>
      )}
      {pending === null && <div className="card empty">{i18n.t("share-nothing")}</div>}
      {pending && (
        <div className="card">
          <h3>{i18n.t("share-list", { count: pending.addresses })}</h3>
          <textarea className="textarea" readOnly value={pending.list} aria-label={i18n.t("share-list", { count: pending.addresses })} />
          <div className="row">
            <button className="button primary" onClick={() => void copy()}>
              <Icon name={copied ? "checkmark" : "copy"} size={16} />
              {i18n.t(copied ? "share-copied" : "share-copy")}
            </button>
            <button className="button" onClick={() => void handOver()}>
              {i18n.t("share-mark-handed-over")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function ReceivePanel() {
  const i18n = useI18n();
  const { refreshBadges } = useApp();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [result, setResult] = useState<Received | null>(null);

  const receive = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      setResult(await api.receiveList(text));
      refreshBadges();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="item-body" style={{ gap: 14 }}>
      <p className="muted">{i18n.t("receive-explain")}</p>
      <div className="card">
        <label className="field">
          <span>{i18n.t("receive-list")}</span>
          <textarea className="textarea" value={text} onChange={(e) => setText(e.target.value)} spellCheck={false} placeholder="ATENA-NAVRHY 1" />
        </label>
        <div className="row">
          <button className="button primary" disabled={busy || !text.trim()} onClick={() => void receive()}>
            {busy && <div className="spinner" />}
            {i18n.t("receive-action")}
          </button>
          {busy && <span className="muted small">{i18n.t("import-busy")}</span>}
        </div>
      </div>
      <ErrorNotice error={error} />
      {result && result.status !== "read" && (
        <div className="notice error">
          <Icon name="info" />
          <span>{i18n.label("receive", result.status, { list: result.list ?? "" })}</span>
        </div>
      )}
      {result && result.status === "read" && (
        <>
          <div className="notice ok" role="status">
            <Icon name="checkmark" />
            <span>{i18n.t("import-done", { count: result.proposed.length })}</span>
          </div>
          <ProposedTable proposed={result.proposed} rejected={result.rejected} />
        </>
      )}
    </div>
  );
}
