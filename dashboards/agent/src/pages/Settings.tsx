import { useEffect, useState } from 'react';
import { api, ApiError, type DbStats } from '../lib/api';

export function Settings({ onLogout }: { onLogout: () => void }) {
  const [stats, setStats] = useState<DbStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = async () => {
    try {
      const s = await api.get<DbStats>('/api/db/stats');
      setStats(s);
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to load stats');
    }
  };

  useEffect(() => {
    load();
  }, []);

  const clearAll = async () => {
    if (!confirm(
      'This will permanently delete all locally stored events and delivery records.\n\n' +
      'This action cannot be undone.\n\n' +
      'Continue?'
    )) return;
    setClearing(true);
    try {
      await api.delete('/api/db/clear');
      await load();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to clear data');
    } finally {
      setClearing(false);
    }
  };

  const logout = async () => {
    if (!confirm('Sign out and delete stored credentials?')) return;
    try {
      await api.post('/api/auth/logout');
      onLogout();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'logout failed');
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div>
      <div className="page-header">
        <h1>Settings</h1>
        <p>Local database management and agent configuration</p>
      </div>

      {error && <div className="card toast toast-error">{error}</div>}

      <div className="card">
        <div className="card-title">Local Database</div>
        <div className="status-row">
          <span className="status-label">Delivery records</span>
          <span className="status-value">{stats?.deliveries_count ?? 0}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Stored events</span>
          <span className="status-value">{stats?.events_count ?? 0}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Database size</span>
          <span className="status-value">{stats ? formatSize(stats.db_size_bytes) : '-'}</span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Configuration Paths</div>
        <div className="status-row">
          <span className="status-label">Config file</span>
          <span className="status-value">~/.config/hookrelay/agent.json</span>
        </div>
        <div className="status-row">
          <span className="status-label">Token storage</span>
          <span className="status-value">OS keychain (or ~/.config/hookrelay/token)</span>
        </div>
        <div className="status-row">
          <span className="status-label">Local database</span>
          <span className="status-value">~/.local/share/hookrelay/agent.db</span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Danger Zone</div>
        <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)', marginBottom: 'var(--space-3)' }}>
          Clearing the local database removes all stored events and delivery history.
          This does not affect data on the server. The agent will continue running.
        </p>
        <button className="btn btn-danger" onClick={clearAll} disabled={clearing}>
          {clearing ? 'Clearing...' : 'Clear Local Database'}
        </button>
      </div>

      <div className="card">
        <div className="card-title">Sign Out</div>
        <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)', marginBottom: 'var(--space-3)' }}>
          Sign out to delete the stored agent token from this machine. You will need
          to log in again to reconnect to the server.
        </p>
        <button className="btn btn-danger" onClick={logout}>
          Sign Out
        </button>
      </div>
    </div>
  );
}
