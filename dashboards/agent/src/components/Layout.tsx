import { NavLink, Outlet } from 'react-router-dom';

const navItems = [
  { path: '/', label: 'Connection', icon: 'link' },
  { path: '/targets', label: 'Targets', icon: 'target' },
  { path: '/events', label: 'Events', icon: 'inbox' },
  { path: '/history', label: 'History', icon: 'clock' },
  { path: '/settings', label: 'Settings', icon: 'gear' },
];

export function Layout({ children }: { children?: React.ReactNode }) {
  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="sidebar-brand">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M4 6h16M4 12h10M4 18h7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
            <circle cx="19" cy="18" r="3" stroke="var(--accent)" strokeWidth="2" fill="none" />
          </svg>
          <span>HookRelay</span>
          <span className="sidebar-badge">Agent</span>
        </div>
        <nav className="sidebar-nav">
          {navItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) => isActive ? 'nav-item active' : 'nav-item'}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <main className="content">
        {children ?? <Outlet />}
      </main>
    </div>
  );
}
