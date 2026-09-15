import { useState } from 'react';
import { copyToClipboard } from '../lib/format';
import './CopyButton.css';

interface Props {
  value: string;
  label?: string;
  /** Render as a compact icon-only button. Defaults to false. */
  compact?: boolean;
  className?: string;
}

export function CopyButton({ value, label, compact = false, className = '' }: Props) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await copyToClipboard(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1400);
    } catch {
      // ignore
    }
  }

  return (
    <button
      type="button"
      className={`copy-btn ${compact ? 'copy-btn-compact' : ''} ${className}`}
      onClick={handleCopy}
      title={copied ? 'Copied' : 'Copy'}
      aria-label={copied ? 'Copied' : 'Copy'}
    >
      {copied ? (
        <span className="copy-btn-check" aria-hidden="true" />
      ) : (
        <span className="copy-btn-icon" aria-hidden="true" />
      )}
      {!compact && <span className="copy-btn-text">{copied ? 'Copied' : label ?? 'Copy'}</span>}
    </button>
  );
}
