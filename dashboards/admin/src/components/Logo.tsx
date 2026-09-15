// HookRelay mark. Matches dashboards/admin/public/favicon.svg and docs/logo.svg.
// Inline so it can be reused in the sidebar, login, and dialogs without an
// extra network request or a CSS placeholder square.

interface Props {
  size?: number;
  withWordmark?: boolean;
  subtle?: boolean;
}

export function Logo({ size = 24, withWordmark = false, subtle = false }: Props) {
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: withWordmark ? 8 : 0,
      }}
    >
      <svg
        width={size}
        height={size}
        viewBox="0 0 32 32"
        fill="none"
        aria-hidden="true"
        focusable="false"
      >
        <rect
          width="32"
          height="32"
          rx="6"
          fill={subtle ? 'transparent' : '#0b0d0e'}
          stroke={subtle ? 'var(--border)' : 'transparent'}
          strokeWidth={subtle ? 1 : 0}
        />
        <path
          d="M8 10h6a4 4 0 0 1 4 4v0a4 4 0 0 1-4 4h-2"
          stroke="#e6e8ea"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        <circle cx="22" cy="14" r="3" stroke="#4ade80" strokeWidth="2" fill="none" />
        <path d="M19 14h-1" stroke="#4ade80" strokeWidth="2" strokeLinecap="round" />
      </svg>
      {withWordmark ? (
        <span
          style={{
            fontFamily: 'var(--font-sans)',
            fontSize: 15,
            fontWeight: 600,
            letterSpacing: '-0.01em',
            color: 'var(--text)',
          }}
        >
          HookRelay
        </span>
      ) : null}
    </span>
  );
}
