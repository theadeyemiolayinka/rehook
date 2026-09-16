// Minimal JSON syntax highlighting without a dependency. Tokenizes a
// JSON string (already pretty-printed) into spans with class names.
// Only used for the formatted view; raw shows the body verbatim.

import type { ReactNode } from 'react';

const TOKEN_RE =
  /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|\b(true|false)\b|\b(null)\b|([{}[\],:])|(\s+)/g;

export function highlightJson(text: string): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let key = 0;
  for (const m of text.matchAll(TOKEN_RE)) {
    const idx = m.index ?? 0;
    if (idx > last) {
      out.push(text.slice(last, idx));
    }
    const [full, str, colon, num, bool, nul, punct] = m;
    if (str !== undefined) {
      out.push(
        colon !== undefined ? (
          <span key={key++} className="json-key">
            {str}
          </span>
        ) : (
          <span key={key++} className="json-string">
            {str}
          </span>
        ),
      );
      if (colon !== undefined) {
        out.push(
          <span key={key++} className="json-punct">
            {colon}
          </span>,
        );
      }
    } else if (num !== undefined) {
      out.push(
        <span key={key++} className="json-number">
          {num}
        </span>,
      );
    } else if (bool !== undefined) {
      out.push(
        <span key={key++} className="json-bool">
          {bool}
        </span>,
      );
    } else if (nul !== undefined) {
      out.push(
        <span key={key++} className="json-null">
          {nul}
        </span>,
      );
    } else if (punct !== undefined) {
      out.push(
        <span key={key++} className="json-punct">
          {punct}
        </span>,
      );
    } else {
      out.push(full);
    }
    last = idx + full.length;
  }
  if (last < text.length) {
    out.push(text.slice(last));
  }
  return out;
}
