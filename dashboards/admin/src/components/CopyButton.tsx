import { useState } from 'react';
import { copyToClipboard } from '../lib/format';
import './CopyButton.css';

interface Props {
  value: string;
  label?: string;
  className?: string;
}

export function CopyButton({ value, label, className = '' }: Props) {
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
      className={`copy-btn ${className}`}
      onClick={handleCopy}
      title="Copy"
    >
      {copied ? 'Copied' : label ?? 'Copy'}
    </button>
  );
}
