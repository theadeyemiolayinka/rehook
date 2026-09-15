import { useEffect, useRef, type ReactNode } from 'react';
import { IconClose } from './Icons';
import './Dialog.css';

interface Props {
  open: boolean;
  title: string;
  description?: ReactNode;
  onClose: () => void;
  children?: ReactNode;
  footer?: ReactNode;
}

export function Dialog({
  open,
  title,
  description,
  onClose,
  children,
  footer,
}: Props) {
  const panelRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  const prevOpenRef = useRef(false);

  // Keep the latest onClose in a ref so the keydown listener always calls
  // the current handler without re-subscribing on every render.
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  // Only focus when the dialog transitions from closed to open. This prevents
  // the focus from being yanked back to the first input on every keystroke
  // when the parent re-renders with a new onClose closure.
  useEffect(() => {
    if (!open) {
      prevOpenRef.current = false;
      return;
    }
    const justOpened = !prevOpenRef.current;
    prevOpenRef.current = true;

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onCloseRef.current();
      }
    };
    document.addEventListener('keydown', onKey);

    let t: number | undefined;
    if (justOpened) {
      t = window.setTimeout(() => {
        const focusable = panelRef.current?.querySelector<HTMLElement>(
          'input, textarea, select, button, [tabindex]:not([tabindex="-1"])',
        );
        focusable?.focus();
      }, 0);
    }

    return () => {
      document.removeEventListener('keydown', onKey);
      if (t !== undefined) window.clearTimeout(t);
    };
  }, [open]);

  // Lock body scroll while open.
  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = prev;
    };
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="dialog-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        ref={panelRef}
      >
        <div className="dialog-header">
          <div className="dialog-heading">
            <h2 className="dialog-title">{title}</h2>
            {description ? (
              <div className="dialog-description">{description}</div>
            ) : null}
          </div>
          <button
            type="button"
            className="dialog-close"
            onClick={onClose}
            aria-label="Close dialog"
          >
            <IconClose size={16} />
          </button>
        </div>
        {children ? <div className="dialog-body">{children}</div> : null}
        {footer ? <div className="dialog-footer">{footer}</div> : null}
      </div>
    </div>
  );
}

interface ConfirmProps {
  open: boolean;
  title: string;
  description: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  loading?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  destructive = false,
  loading = false,
  onConfirm,
  onCancel,
}: ConfirmProps) {
  return (
    <Dialog
      open={open}
      title={title}
      description={description}
      onClose={onCancel}
      footer={
        <>
          <button
            type="button"
            className="btn btn-sm"
            onClick={onCancel}
            disabled={loading}
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            className={`btn btn-sm ${destructive ? 'btn-danger' : 'btn-primary'}`}
            onClick={onConfirm}
            disabled={loading}
          >
            {loading ? 'Working...' : confirmLabel}
          </button>
        </>
      }
    />
  );
}
