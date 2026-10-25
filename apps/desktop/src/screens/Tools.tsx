import { ScopeSwitch } from "../components/Scope";
import { ToolList, inScope, useTools } from "../components/Tools";
import { useApp } from "../context";
import { useI18n } from "../i18n";

/** Dictionaries, corpora and applications of the register: linked and described, never crawled. */
export function ToolsScreen() {
  const i18n = useI18n();
  const { settings } = useApp();
  const tools = inScope(useTools(), settings);
  return (
    <div className="page">
      <div className="page-head">
        <h1>{i18n.t("nav-tools")}</h1>
        <p className="muted">{i18n.t("tools-intro")}</p>
      </div>
      <ScopeSwitch />
      {tools.length > 0 ? (
        <ToolList tools={tools} title={i18n.t("tools-title")} />
      ) : (
        <div className="card empty">{i18n.t("tools-none")}</div>
      )}
    </div>
  );
}
