import { useState } from 'react';
import { NavLink, Outlet, useLocation } from 'react-router-dom';
import { Logo } from './Logo';
import {
  IconConnection,
  IconTargets,
  IconRoute,
  IconEvents,
  IconHistory,
  IconSettings,
} from './Icons';

const navItems = [
  { path: '/', label: 'Connection', icon: IconConnection, end: true },
  { path: '/targets', label: 'Targets', icon: IconTargets, end: false },
  { path: '/routes', label: 'Routes', icon: IconRoute, end: false },
  { path: '/events', label: 'Events', icon: IconEvents, end: false },
  { path: '/history', label: 'History', icon: IconHistory, end: false },
  { path: '/settings', label: 'Settings', icon: IconSettings, end: false },
];

const labelMap: Record<string, string> = {
  '': 'Connection',
  targets: 'Targets',
  routes: 'Routes',
  events: 'Events',
  history: 'History',
  settings: 'Settings',
};

function getBreadcrumbs(pathname: string): { label: string; path: string }[] {
  const parts = pathname.split('/').filter(Boolean);
  if (parts.length === 0) {
    return [{ label: 'Connection', path: '/' }];
  }
  const crumbs: { label: string; path: string }[] = [];
  let acc = '';
  for (const part of parts) {
    acc += '/' + part;
    const label = labelMap[part] || part;
    crumbs.push({ label, path: acc });
  }
  return crumbs;
}

export function Layout({ children }: { children?: React.ReactNode }) {
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const breadcrumbs = getBreadcrumbs(location.pathname);

  const sidebarContent = (
    <>
      <div className="sidebar-brand">
        <Logo size={28} />
        <span className="sidebar-brand-name">Rehook</span>
      </div>
      <nav className="sidebar-nav" aria-label="Primary navigation">
        {navItems.map((item) => {
          const Icon = item.icon;
          return (
            <NavLink
              key={item.path}
              to={item.path}
              end={item.end}
              className={({ isActive }) =>
                isActive ? 'nav-item active' : 'nav-item'
              }
            >
              <span className="nav-item-icon" aria-hidden="true">
                <Icon size={16} />
              </span>
              <span className="nav-item-label">{item.label}</span>
            </NavLink>
          );
        })}
      </nav>
    </>
  );

  return (
    <div className="layout">
      <aside className="sidebar">{sidebarContent}</aside>
      {mobileOpen && (
        <>
          <div
            className="sidebar-overlay"
            onClick={() => setMobileOpen(false)}
          />
          <aside className="sidebar-mobile">{sidebarContent}</aside>
        </>
      )}
      <div className="layout-main">
        <header className="topbar">
          <button
            type="button"
            className="topbar-menu"
            onClick={() => setMobileOpen(true)}
            aria-label="Open navigation"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M2 4h12M2 8h12M2 12h12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
          <nav className="breadcrumbs" aria-label="Breadcrumb">
            {breadcrumbs.map((crumb, i) => (
              <span key={crumb.path} className="breadcrumb-item">
                {i > 0 && <span className="breadcrumb-sep">/</span>}
                {i === breadcrumbs.length - 1 ? (
                  <span className="breadcrumb-current">{crumb.label}</span>
                ) : (
                  <NavLink to={crumb.path} className="breadcrumb-link">
                    {crumb.label}
                  </NavLink>
                )}
              </span>
            ))}
          </nav>
        </header>
        <main className="content">
          {children ?? <Outlet />}
        </main>
      </div>
    </div>
  );
}
