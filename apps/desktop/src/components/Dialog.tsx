import { useEffect, useRef, type ReactNode } from "react";

import { useI18n } from "../i18n";
import { Icon } from "../icons";

/** A modal dialog: Escape or the close button closes it; focus moves into it and back. */
export function Dialog({
  title,
  onClose,
  narrow = false,
  children,
}: {
  title: string;
  onClose: () => void;
  narrow?: boolean;
  children: ReactNode;
}) {
  const i18n = useI18n();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      before?.focus();
    };
  }, [onClose]);
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        className={narrow ? "dialog narrow" : "dialog"}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={ref}
      >
        <div className="row spread">
          <h2>{title}</h2>
          <button className="button link" onClick={onClose} aria-label={i18n.t("action-close")}>
            <Icon name="dismiss" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
