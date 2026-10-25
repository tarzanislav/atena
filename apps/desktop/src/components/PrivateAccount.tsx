import { useState, type FormEvent } from "react";

import { api, type PrivateStatus } from "../api";
import { useApp } from "../context";
import { useI18n } from "../i18n";
import { Icon } from "../icons";
import { Dialog } from "./Dialog";
import { ErrorNotice } from "./ErrorNotice";

/** Shows the recovery key once: it can be copied, and the dialog closes only after it was kept. */
export function RecoveryKeyDialog({ recoveryKey, onClose }: { recoveryKey: string; onClose: () => void }) {
  const i18n = useI18n();
  const [kept, setKept] = useState(false);
  const [copied, setCopied] = useState(false);
  return (
    <Dialog title={i18n.t("private-recovery-title")} onClose={() => kept && onClose()} narrow>
      <p>{i18n.t("private-recovery-text")}</p>
      <div className="recovery-key" aria-label={i18n.t("private-recovery-title")}>
        {recoveryKey}
      </div>
      <div className="row">
        <button
          className="button"
          onClick={() => {
            void navigator.clipboard.writeText(recoveryKey).then(() => setCopied(true));
          }}
        >
          <Icon name="copy" size={16} />
          {copied ? i18n.t("private-recovery-copied") : i18n.t("private-recovery-copy")}
        </button>
      </div>
      <label className="check">
        <input type="checkbox" checked={kept} onChange={(e) => setKept(e.target.checked)} />
        <span>{i18n.t("private-recovery-kept")}</span>
      </label>
      <div className="row" style={{ justifyContent: "flex-end" }}>
        <button className="button primary" disabled={!kept} onClick={onClose}>
          {i18n.t("private-recovery-done")}
        </button>
      </div>
    </Dialog>
  );
}

/** Asks for the name and password of the private account; on success asks whether to stay signed in. */
export function UnlockDialog({
  onClose,
  onUnlocked,
}: {
  onClose: () => void;
  onUnlocked: (status: PrivateStatus, remember: boolean) => void;
}) {
  const i18n = useI18n();
  const { privateStatus } = useApp();
  const [name, setName] = useState(privateStatus.name ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [opened, setOpened] = useState<PrivateStatus | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      setOpened(await api.privateUnlock(name, password));
      setPassword("");
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  if (opened) {
    return (
      <Dialog title={i18n.t("private-remember-title")} onClose={() => onUnlocked(opened, false)} narrow>
        <p>{i18n.t("private-remember-text")}</p>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button className="button" onClick={() => onUnlocked(opened, false)}>
            {i18n.t("private-remember-no")}
          </button>
          <button className="button primary" autoFocus onClick={() => onUnlocked(opened, true)}>
            {i18n.t("private-remember-yes")}
          </button>
        </div>
        {opened.rememberAvailable && !opened.remembered && (
          <PersistentSignIn onDone={(status) => onUnlocked(status, true)} />
        )}
      </Dialog>
    );
  }
  return (
    <Dialog title={i18n.t("private-unlock-title")} onClose={onClose} narrow>
      <form className="item-body" style={{ gap: 12 }} onSubmit={(e) => void submit(e)}>
        <label className="field">
          <span>{i18n.t("private-name")}</span>
          <input className="input" value={name} autoComplete="username" onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span>{i18n.t("private-password")}</span>
          <input
            className="input"
            type="password"
            value={password}
            autoFocus
            autoComplete="current-password"
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <ErrorNotice error={error} />
        <div className="row" style={{ justifyContent: "flex-end" }}>
          {busy && <div className="spinner" />}
          <button className="button primary" type="submit" disabled={busy || !name || !password}>
            <Icon name="lockOpen" size={16} />
            {i18n.t("private-unlock")}
          </button>
        </div>
      </form>
    </Dialog>
  );
}

/** Turns the persistent sign-in on – only after the user ticked that they do it at their own risk. */
function PersistentSignIn({ onDone }: { onDone: (status: PrivateStatus) => void }) {
  const i18n = useI18n();
  const [accepted, setAccepted] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const enable = async () => {
    setBusy(true);
    setError(null);
    try {
      onDone(await api.privateRemember());
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="remember-box">
      <strong>{i18n.t("private-remember-forever-title")}</strong>
      <p className="small">{i18n.t("private-remember-forever-text")}</p>
      <label className="check">
        <input type="checkbox" checked={accepted} onChange={(e) => setAccepted(e.target.checked)} />
        <span>{i18n.t("private-remember-risk")}</span>
      </label>
      <ErrorNotice error={error} />
      <div className="row" style={{ justifyContent: "flex-end" }}>
        {busy && <div className="spinner" />}
        <button className="button" disabled={!accepted || busy} onClick={() => void enable()}>
          <Icon name="lockOpen" size={16} />
          {i18n.t("private-remember-forever")}
        </button>
      </div>
    </div>
  );
}

/** The private account in the settings: make it, change its password, set a new one with the
 * recovery key, or turn the persistent sign-in on and off. */
export function PrivateAccountCard() {
  const i18n = useI18n();
  const { privateStatus, setPrivateStatus } = useApp();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [again, setAgain] = useState("");
  const [recovery, setRecovery] = useState("");
  const [shownKey, setShownKey] = useState<string | null>(null);
  const [mode, setMode] = useState<"none" | "password" | "recover" | "remember">("none");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    setDone(false);
    try {
      await work();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  const mismatch = again.length > 0 && password !== again;

  const create = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      const created = await api.privateCreate(name, password);
      setPassword("");
      setAgain("");
      setShownKey(created.recoveryKey);
      setPrivateStatus(created.status);
    });
  };

  return (
    <div className="card private-card">
      <h2 className="row" style={{ gap: 8 }}>
        <Icon name="lockClosed" />
        {i18n.t("private-account-title")}
      </h2>
      <p className="small">{i18n.t("private-account-intro")}</p>
      <ErrorNotice error={error} />

      {!privateStatus.exists && (
        <form className="item-body" style={{ gap: 10 }} onSubmit={create}>
          <label className="field">
            <span>{i18n.t("private-name")}</span>
            <input className="input" value={name} maxLength={40} autoComplete="username" onChange={(e) => setName(e.target.value)} />
          </label>
          <label className="field">
            <span>{i18n.t("private-password")}</span>
            <input className="input" type="password" value={password} autoComplete="new-password" onChange={(e) => setPassword(e.target.value)} />
          </label>
          <label className="field">
            <span>{i18n.t("private-password-again")}</span>
            <input className="input" type="password" value={again} autoComplete="new-password" onChange={(e) => setAgain(e.target.value)} />
          </label>
          <p className="small muted">{i18n.t("private-password-rules")}</p>
          {mismatch && <p className="small" style={{ color: "var(--red-ink)" }}>{i18n.t("private-password-mismatch")}</p>}
          <div>
            <button className="button primary" type="submit" disabled={busy || !name || !password || password !== again}>
              {i18n.t("private-create")}
            </button>
          </div>
        </form>
      )}

      {privateStatus.exists && (
        <>
          <p>
            {i18n.t("private-account-of", { name: privateStatus.name ?? "" })}{" "}
            <strong>{privateStatus.unlocked ? i18n.t("private-state-open") : i18n.t("private-state-locked")}</strong>
          </p>
          <div className="row">
            <button className="button" disabled={!privateStatus.unlocked} onClick={() => setMode(mode === "password" ? "none" : "password")}>
              {i18n.t("private-change-password")}
            </button>
            <button className="button" onClick={() => setMode(mode === "recover" ? "none" : "recover")}>
              {i18n.t("private-forgot")}
            </button>
            {privateStatus.unlocked && (
              <button className="button" onClick={() => void run(async () => setPrivateStatus(await api.privateLock()))}>
                <Icon name="lockClosed" size={16} />
                {i18n.t("private-lock")}
              </button>
            )}
          </div>
          {privateStatus.remembered && (
            <div className="remember-box">
              <p className="row small" style={{ gap: 8 }}>
                <Icon name="lockOpen" size={16} />
                <strong>{i18n.t("private-remember-on")}</strong>
              </p>
              <div>
                <button className="button" disabled={busy} onClick={() => void run(async () => setPrivateStatus(await api.privateForget()))}>
                  {i18n.t("private-remember-off")}
                </button>
              </div>
            </div>
          )}
          {!privateStatus.remembered && privateStatus.rememberAvailable && privateStatus.unlocked && (
            <div>
              <button className="button link" onClick={() => setMode(mode === "remember" ? "none" : "remember")}>
                {i18n.t("private-remember-enable")}
              </button>
            </div>
          )}
          {mode === "remember" && !privateStatus.remembered && privateStatus.unlocked && (
            <PersistentSignIn
              onDone={(status) => {
                setPrivateStatus(status);
                setMode("none");
              }}
            />
          )}
          {mode === "password" && (
            <form
              className="item-body"
              style={{ gap: 10 }}
              onSubmit={(e) => {
                e.preventDefault();
                void run(async () => {
                  await api.privateChangePassword(recovery, password);
                  setRecovery("");
                  setPassword("");
                  setAgain("");
                  setMode("none");
                  setDone(true);
                });
              }}
            >
              <label className="field">
                <span>{i18n.t("private-password-old")}</span>
                <input className="input" type="password" value={recovery} autoComplete="current-password" onChange={(e) => setRecovery(e.target.value)} />
              </label>
              <label className="field">
                <span>{i18n.t("private-password-new")}</span>
                <input className="input" type="password" value={password} autoComplete="new-password" onChange={(e) => setPassword(e.target.value)} />
              </label>
              <label className="field">
                <span>{i18n.t("private-password-again")}</span>
                <input className="input" type="password" value={again} autoComplete="new-password" onChange={(e) => setAgain(e.target.value)} />
              </label>
              {mismatch && <p className="small" style={{ color: "var(--red-ink)" }}>{i18n.t("private-password-mismatch")}</p>}
              <div>
                <button className="button primary" type="submit" disabled={busy || !recovery || !password || password !== again}>
                  {i18n.t("private-change-password")}
                </button>
              </div>
            </form>
          )}
          {mode === "recover" && (
            <form
              className="item-body"
              style={{ gap: 10 }}
              onSubmit={(e) => {
                e.preventDefault();
                void run(async () => {
                  setPrivateStatus(await api.privateRecover(recovery, password));
                  setRecovery("");
                  setPassword("");
                  setAgain("");
                  setMode("none");
                  setDone(true);
                });
              }}
            >
              <p className="small muted">{i18n.t("private-forgot-text")}</p>
              <label className="field">
                <span>{i18n.t("private-recovery-title")}</span>
                <input className="input" value={recovery} autoComplete="off" spellCheck={false} placeholder="ATENA-XXXX-XXXX-…" onChange={(e) => setRecovery(e.target.value)} />
              </label>
              <label className="field">
                <span>{i18n.t("private-password-new")}</span>
                <input className="input" type="password" value={password} autoComplete="new-password" onChange={(e) => setPassword(e.target.value)} />
              </label>
              <label className="field">
                <span>{i18n.t("private-password-again")}</span>
                <input className="input" type="password" value={again} autoComplete="new-password" onChange={(e) => setAgain(e.target.value)} />
              </label>
              {mismatch && <p className="small" style={{ color: "var(--red-ink)" }}>{i18n.t("private-password-mismatch")}</p>}
              <div>
                <button className="button primary" type="submit" disabled={busy || !recovery || !password || password !== again}>
                  {i18n.t("private-set-password")}
                </button>
              </div>
            </form>
          )}
          {done && (
            <p className="row small" role="status" style={{ color: "var(--green-ink)" }}>
              <Icon name="checkmark" size={16} />
              {i18n.t("private-password-changed")}
            </p>
          )}
        </>
      )}
      {busy && <div className="spinner" />}
      {shownKey && <RecoveryKeyDialog recoveryKey={shownKey} onClose={() => setShownKey(null)} />}
    </div>
  );
}
