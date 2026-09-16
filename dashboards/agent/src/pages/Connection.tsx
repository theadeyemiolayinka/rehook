import { useEffect, useState } from 'react';
import { api, ApiError, type ConnectionStatus, type ServerProject } from '../lib/api';
import { ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconRefresh, IconLogout } from '../components/Icons';

export function Connection({
  status,
  onRefresh,
}: {
  status: ConnectionStatus | null;
  onRefresh: () => void;
}) {
  const [logoutOpen, setLogoutOpen] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const [endpoints, setEndpoints] = useState<Record<string, { name: string; projectName: string }>>({});
  const toast = useToast();

  useEffect(() => {
    api.get<{ projects: ServerProject[] }>('/api/projects')
      .then((res) => {
        const map: Record<string, { name: string; projectName: string }> = {};
        for (const p of res.projects ?? []) {
          for (const e of p.endpoints ?? []) {
            map[e.id] = { name: e.name, projectName: p.name };
          }
        }
        setEndpoints(map);
      })
      .catch(() => setEndpoints({}));
  }, [status?.connected]);

  const logout = async () => {
    setLoggingOut(true);
    try {
      await api.post('/api/auth/logout');
      toast.show('Signed out', 'success');
      setLogoutOpen(false);
      onRefresh();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Logout failed',
        'error',
      );
    } finally {
      setLoggingOut(false);
    }
  };

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <h1>Connection</h1>
            <p>Server connection and agent identity.</p>
          </div>
          <button
            type="button"
            className="btn btn-sm"
            onClick={onRefresh}
          >
            <IconRefresh size={14} /> Refresh
          </button>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Server</div>
        <div className="status-row">
          <span className="status-label">Status</span>
          <span
            className={`status-dot ${status?.connected ? 'connected' : 'disconnected'}`}
            aria-hidden="true"
          />
          <span className="status-value">
            {status?.connected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Server URL</span>
          <span className="status-value">
            {status?.server_url ?? 'not configured'}
          </span>
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
        <div className="card-title">Agent identity</div>
        <div className="status-row">
          <span className="status-label">Agent ID</span>
          <span className="status-value mono">
            {status?.agent_id ?? 'not configured'}
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Agent name</span>
          <span className="status-value">
            {status?.agent_name ?? 'not configured'}
          </span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Subscribed endpoints</div>
        {status?.subscribed_endpoints && status.subscribed_endpoints.length > 0 ? (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Endpoint</th>
                  <th scope="col">Endpoint ID</th>
                </tr>
              </thead>
              <tbody>
                {status.subscribed_endpoints.map((eid) => {
                  const ep = endpoints[eid];
                  return (
                    <tr key={eid}>
                      <td>
                        {ep ? (
                          <span>{ep.projectName} / {ep.name}</span>
                        ) : (
                          <span className="text-tertiary">unknown</span>
                        )}
                      </td>
                      <td className="mono text-tertiary">{eid}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="empty-state">
            No endpoint subscriptions. Add routes in the Routes page to start
            receiving events.
          </div>
        )}
      </div>

      <div className="card">
        <div className="card-title">Session</div>
        <p className="card-help">
          Sign out to delete the stored agent token from this machine. You will
          need to log in again to reconnect.
        </p>
        <button
          className="btn btn-danger"
          onClick={() => setLogoutOpen(true)}
        >
          <IconLogout size={14} /> Sign out
        </button>
      </div>

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
