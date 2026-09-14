import { api, ApiError, type ConnectionStatus } from '../lib/api';

export function Connection({
  status,
  onRefresh,
}: {
  status: ConnectionStatus | null;
  onRefresh: () => void;
}) {
  const logout = async () => {
    if (!confirm('Sign out and delete stored credentials?')) return;
    try {
      await api.post('/api/auth/logout');
      onRefresh();
    } catch (e) {
      alert(e instanceof ApiError ? e.message : 'logout failed');
    }
  };

  return (
    <div>
      <div className="page-header">
        <h1>Connection</h1>
        <p>Server connection and agent identity</p>
      </div>

      <div className="card">
        <div className="card-title">Server</div>
        <div className="status-row">
          <span className="status-label">Status</span>
          <span className={`status-dot ${status?.connected ? 'connected' : 'disconnected'}`} />
          <span className="status-value">
            {status?.connected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Server URL</span>
          <span className="status-value">{status?.server_url ?? 'not configured'}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Authenticated</span>
          <span className="status-value">
            {status?.authenticated ? 'Yes' : 'No'}
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Last connected</span>
          <span className="status-value">
            {status?.last_connected_at ?? 'never'}
          </span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Agent Identity</div>
        <div className="status-row">
          <span className="status-label">Agent ID</span>
          <span className="status-value">{status?.agent_id ?? 'not configured'}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Agent name</span>
          <span className="status-value">{status?.agent_name ?? 'not configured'}</span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Subscribed Projects</div>
        {status?.subscribed_projects && status.subscribed_projects.length > 0 ? (
          <table className="table">
            <tbody>
              {status.subscribed_projects.map((pid) => (
                <tr key={pid}>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>{pid}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="empty-state">No project subscriptions. Add routes in the Targets page.</div>
        )}
      </div>

      <div className="card">
        <div className="card-title">Session</div>
        <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)', marginBottom: 'var(--space-3)' }}>
          Sign out to delete the stored agent token from this machine. You will need to log in again to reconnect.
        </p>
        <button className="btn btn-danger" onClick={logout}>
          Sign Out
        </button>
      </div>
    </div>
  );
}
