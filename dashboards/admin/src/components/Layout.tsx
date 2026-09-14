import { type ReactNode } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { api } from '../lib/api';
import { useToast } from '../components/Toast';
import './Layout.css';

interface Props {
  children: ReactNode;
}

const NAV = [
  { to: '/', label: 'Overview', end: true },
  { to: '/projects', label: 'Projects' },
  { to: '/endpoints', label: 'Endpoints' },
  { to: '/events', label: 'Events' },
  { to: '/agents', label: 'Agents' },
  { to: '/settings', label: 'Settings' },
];

export function Layout({ children }: Props) {
  const navigate = useNavigate();
  const toast = useToast();

  async function handleLogout() {
    try {
      await api.post('/api/auth/logout');
      navigate('/login');
    } catch {
      toast.show('Logout failed', 'error');
    }
  }

  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="sidebar-brand">
          <span className="sidebar-mark" />
          HookRelay
        </div>
        <nav className="sidebar-nav">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `sidebar-link ${isActive ? 'active' : ''}`
              }
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-footer">
          <button className="sidebar-logout" onClick={handleLogout}>
            Sign out
          </button>
        </div>
      </aside>
      <main className="main">{children}</main>
    </div>
  );
}
