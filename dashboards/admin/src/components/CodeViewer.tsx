import { useMemo, useState } from 'react';
import { CopyButton } from './CopyButton';
import './CodeViewer.css';

interface Props {
  body: string; // decoded body text
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
  const [mode, setMode] = useState<Mode>('formatted');

  // Render limit to avoid freezing the browser on large payloads.
  const RENDER_LIMIT = 200_000;
  const display = useMemo(() => {
    const src = mode === 'formatted' && formatted ? formatted : body;
    if (src.length <= RENDER_LIMIT) return { text: src, truncated: false };
    return { text: src.slice(0, RENDER_LIMIT), truncated: true };
  }, [mode, formatted, body]);

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
        <CopyButton value={mode === 'formatted' && formatted ? formatted : body} />
      </div>
      <pre className="code-viewer-body">
        <code>{display.text}</code>
      </pre>
      {display.truncated ? (
        <div className="code-viewer-truncated">
          Output truncated. Use the raw body endpoint or copy to inspect the full payload.
        </div>
      ) : null}
    </div>
  );
}
