import { useMemo, useState } from 'react';
import { IconCopy } from './Icons';
import { highlightJson } from '../lib/jsonHighlight';
import './CodeViewer.css';

interface Props {
  body: string;
  contentType?: string | null;
}

type Mode = 'formatted' | 'raw';

function tryFormatJson(text: string): string | null {
  try {
    const parsed = JSON.parse(text);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return null;
  }
}

export function CodeViewer({ body, contentType }: Props) {
  const isJson =
    contentType?.includes('json') ||
    body.trimStart().startsWith('{') ||
    body.trimStart().startsWith('[');

  const formatted = useMemo(() => (isJson ? tryFormatJson(body) : null), [body, isJson]);
  const [mode, setMode] = useState<Mode>(isJson && formatted ? 'formatted' : 'raw');
  const [copied, setCopied] = useState(false);

  const RENDER_LIMIT = 200_000;
  const display = useMemo(() => {
    const src = mode === 'formatted' && formatted ? formatted : body;
    if (src.length <= RENDER_LIMIT) return { text: src, truncated: false };
    return { text: src.slice(0, RENDER_LIMIT), truncated: true };
  }, [mode, formatted, body]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(display.text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // fallback: select text
    }
  };

  return (
    <div className="code-viewer">
      <div className="code-viewer-toolbar">
        <div className="code-viewer-modes">
          {isJson && formatted ? (
            <>
              <button
                className={`code-mode ${mode === 'formatted' ? 'active' : ''}`}
                onClick={() => setMode('formatted')}
              >
                Formatted
              </button>
              <button
                className={`code-mode ${mode === 'raw' ? 'active' : ''}`}
                onClick={() => setMode('raw')}
              >
                Raw
              </button>
            </>
          ) : (
            <span className="code-mode-label">
              {isJson ? 'Invalid JSON' : 'Plain text'}
            </span>
          )}
        </div>
        <button className="code-viewer-copy" onClick={copy} aria-label="Copy body">
          <IconCopy size={13} /> {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre className="code-viewer-body">
        <code>
          {mode === 'formatted' && formatted && !display.truncated
            ? highlightJson(display.text)
            : display.text}
        </code>
      </pre>
      {display.truncated ? (
        <div className="code-viewer-truncated">
          Output truncated. Copy to inspect the full payload.
        </div>
      ) : null}
    </div>
  );
}
