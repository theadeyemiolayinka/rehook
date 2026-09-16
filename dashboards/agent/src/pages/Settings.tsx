import { useCallback, useEffect, useState } from 'react';
import { api, ApiError, type DbStats } from '../lib/api';
import { ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconRefresh, IconTrash, IconLogout } from '../components/Icons';

export function Settings({ onLogout }: { onLogout: () => void }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<DbStats | null>(null);
  const [clearOpen, setClearOpen] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [logoutOpen, setLogoutOpen] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const s = await api.get<DbStats>('/api/db/stats');
      setStats(s);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load stats');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const clearAll = async () => {
    setClearing(true);
    try {
      await api.delete('/api/db/clear');
      toast.show('Local database cleared', 'success');
      setClearOpen(false);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to clear data',
        'error',
      );
    } finally {
      setClearing(false);
    }
  };

  const logout = async () => {
    setLoggingOut(true);
    try {
      await api.post('/api/auth/logout');
      toast.show('Signed out', 'success');
      setLogoutOpen(false);
      onLogout();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Logout failed',
        'error',
      );
    } finally {
      setLoggingOut(false);
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
        <div className="page-header-row">
          <div>
            <h1>Settings</h1>
            <p>Local database management and agent configuration.</p>
          </div>
          <button type="button" className="btn btn-sm" onClick={load}>
            <IconRefresh size={14} /> Refresh
          </button>
        </div>
      </div>

      {error ? (
        <div className="card error-card">
          <div className="error-title">Unable to load settings</div>
          <div className="error-detail">{error}</div>
          <button className="btn btn-sm" onClick={load}>
            Retry
          </button>
        </div>
      ) : null}

      <div className="card">
        <div className="card-title">Local database</div>
        <div className="status-row">
          <span className="status-label">Stored events</span>
          <span className="status-value">{stats?.events_count ?? 0}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Delivery records</span>
          <span className="status-value">{stats?.deliveries_count ?? 0}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Database size</span>
          <span className="status-value">
            {stats ? formatSize(stats.db_size_bytes) : '-'}
          </span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Configuration paths</div>
        <div className="status-row">
          <span className="status-label">Config file</span>
          <span className="status-value">~/.config/rehook/agent.json</span>
        </div>
        <div className="status-row">
          <span className="status-label">Token storage</span>
          <span className="status-value">
            OS keychain (or ~/.config/rehook/token)
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Local database</span>
          <span className="status-value">~/.local/share/rehook/agent.db</span>
        </div>
      </div>

      <div className="card danger-card">
        <div className="card-title">Clear local database</div>
        <p className="card-help">
          Clearing the local database removes all stored events and delivery
          history. This does not affect data on the server. The agent will
          continue running.
        </p>
        <button
          className="btn btn-danger"
          onClick={() => setClearOpen(true)}
          disabled={clearing || loading}
        >
          <IconTrash size={14} /> Clear local database
        </button>
      </div>

      <div className="card danger-card">
        <div className="card-title">Sign out</div>
        <p className="card-help">
          Sign out to delete the stored agent token from this machine. You will
          need to log in again to reconnect to the server.
        </p>
        <button className="btn btn-danger" onClick={() => setLogoutOpen(true)}>
          <IconLogout size={14} /> Sign out
        </button>
      </div>

      <ConfirmDialog
        open={clearOpen}
        title="Clear local database"
        description="Permanently delete all locally stored events and delivery records? This cannot be undone. Server data is not affected."
        confirmLabel="Clear database"
        destructive
        loading={clearing}
        onConfirm={clearAll}
        onCancel={() => setClearOpen(false)}
      />

      <ConfirmDialog
        open={logoutOpen}
        title="Sign out"
        description="Delete stored credentials and disconnect the agent? You will need to log in again to reconnect."
        confirmLabel="Sign out"
        destructive
        loading={loggingOut}
        onConfirm={logout}
        onCancel={() => setLogoutOpen(false)}
      />
    </div>
  );
}
