import { type ReactNode, useState } from 'react';
import { NavLink, useNavigate, useLocation } from 'react-router-dom';
import { api } from '../lib/api';
import { useToast } from './Toast';
import { Logo } from './Logo';
import {
  IconOverview,
  IconProjects,
  IconEndpoints,
  IconEvents,
  IconAgents,
  IconSettings,
  IconLogout,
} from './Icons';
import './Layout.css';

interface Props {
  children: ReactNode;
}

const NAV = [
  { to: '/', label: 'Overview', icon: IconOverview, end: true },
  { to: '/projects', label: 'Projects', icon: IconProjects, end: false },
  { to: '/endpoints', label: 'Endpoints', icon: IconEndpoints, end: false },
  { to: '/events', label: 'Events', icon: IconEvents, end: false },
  { to: '/agents', label: 'Agents', icon: IconAgents, end: false },
  { to: '/settings', label: 'Settings', icon: IconSettings, end: false },
];

function getBreadcrumbs(pathname: string): { label: string; path: string }[] {
  const crumbs: { label: string; path: string }[] = [];
  const parts = pathname.split('/').filter(Boolean);
  if (parts.length === 0) {
    crumbs.push({ label: 'Overview', path: '/' });
    return crumbs;
  }
  const labelMap: Record<string, string> = {
    projects: 'Projects',
    endpoints: 'Endpoints',
    events: 'Events',
    agents: 'Agents',
    settings: 'Settings',
  };
  let acc = '';
  for (const part of parts) {
    acc += '/' + part;
    const label = labelMap[part] || part;
    crumbs.push({ label, path: acc });
  }
  return crumbs;
}

export function Layout({ children }: Props) {
  const navigate = useNavigate();
  const toast = useToast();
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);

  async function handleLogout() {
    try {
      await api.post('/api/auth/logout');
      navigate('/login');
    } catch {
      toast.show('Logout failed', 'error');
    }
  }

  const breadcrumbs = getBreadcrumbs(location.pathname);

  const sidebarContent = (
    <>
      <div className="sidebar-brand">
        <Logo size={28} />
        <span className="sidebar-brand-name">HookRelay</span>
      </div>
      <nav className="sidebar-nav" aria-label="Primary navigation">
        {NAV.map((item) => {
          const Icon = item.icon;
          return (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `sidebar-link ${isActive ? 'active' : ''}`
              }
            >
              <span className="sidebar-link-icon" aria-hidden="true">
                <Icon size={16} />
              </span>
              <span className="sidebar-link-label">{item.label}</span>
            </NavLink>
          );
        })}
      </nav>
      <div className="sidebar-footer">
        <button
          type="button"
          className="sidebar-logout"
          onClick={handleLogout}
        >
          <span className="sidebar-link-icon" aria-hidden="true">
            <IconLogout size={16} />
          </span>
          <span>Sign out</span>
        </button>
      </div>
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
        <main className="main">{children}</main>
      </div>
    </div>
  );
}
