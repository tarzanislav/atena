import { isAppError } from "../api";
import { useI18n } from "../i18n";
import { Icon } from "../icons";

/** An error in the interface language, with the technical detail for those who look further. */
export function ErrorNotice({ error }: { error: unknown }) {
  const i18n = useI18n();
  if (error == null) return null;
  const code = isAppError(error) ? error.code : "internal";
  const detail = isAppError(error) ? error.detail : String(error);
  return (
    <div className="notice error" role="alert">
      <Icon name="warning" />
      <div className="item-body">
        <span>{i18n.label("error", code)}</span>
        {detail && (
          <details>
            <summary>{i18n.t("error-detail")}</summary>
            <span className="detail">{detail}</span>
          </details>
        )}
      </div>
    </div>
  );
}
