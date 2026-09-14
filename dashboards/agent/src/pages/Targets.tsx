import { useEffect, useState } from 'react';
import { api, ApiError, type Target, type RouteEntry } from '../lib/api';

export function Targets() {
  const [targets, setTargets] = useState<Target[]>([]);
  const [routes, setRoutes] = useState<RouteEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [newId, setNewId] = useState('');
  const [newUrl, setNewUrl] = useState('');
  const [newProjectId, setNewProjectId] = useState('');
  const [newTargetId, setNewTargetId] = useState('');

  const load = async () => {
    try {
      const [targetsResp, routesResp] = await Promise.all([
        api.get<{ targets: Target[] }>('/api/targets'),
        api.get<{ routes: RouteEntry[] }>('/api/routes'),
      ]);
      setTargets(targetsResp.targets);
      setRoutes(routesResp.routes);
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to load');
    }
  };

  useEffect(() => {
    load();
  }, []);

  const addTarget = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newId.trim() || !newUrl.trim()) return;
    try {
      await api.post('/api/targets', { id: newId.trim(), url: newUrl.trim() });
      setNewId('');
      setNewUrl('');
      await load();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to add target');
    }
  };

  const removeTarget = async (id: string) => {
    if (!confirm(`Remove target "${id}"? Routes using this target will also be removed.`)) return;
    try {
      await api.delete(`/api/targets/${encodeURIComponent(id)}`);
      await load();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to remove target');
    }
  };

  const addRoute = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newProjectId.trim() || !newTargetId.trim()) return;
    try {
      await api.post('/api/routes', {
        project_id: newProjectId.trim(),
        target_id: newTargetId.trim(),
      });
      setNewProjectId('');
      setNewTargetId('');
      await load();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to add route');
    }
  };

  const removeRoute = async (projectId: string) => {
    if (!confirm(`Remove route for project "${projectId}"?`)) return;
    try {
      await api.delete(`/api/routes/${encodeURIComponent(projectId)}`);
      await load();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to remove route');
    }
  };

  return (
    <div>
      <div className="page-header">
        <h1>Targets and Routes</h1>
        <p>Local HTTP destinations and project-to-target mappings</p>
      </div>

      {error && <div className="card toast toast-error">{error}</div>}

      <div className="card">
        <div className="card-title">Add Target</div>
        <form onSubmit={addTarget}>
          <div className="form-row">
            <div className="form-field">
              <label>Target ID</label>
              <input
                value={newId}
                onChange={(e) => setNewId(e.target.value)}
                placeholder="myapp"
              />
            </div>
            <div className="form-field">
              <label>Local URL</label>
              <input
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                placeholder="http://localhost:8000/webhook"
              />
            </div>
            <button type="submit" className="btn btn-primary">Add</button>
          </div>
        </form>
      </div>

      <div className="card">
        <div className="card-title">Configured Targets</div>
        {targets.length === 0 ? (
          <div className="empty-state">No targets configured</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>ID</th>
                <th>URL</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {targets.map((t) => (
                <tr key={t.id}>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>{t.id}</td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>{t.url}</td>
                  <td>
                    <button className="btn btn-danger btn-sm" onClick={() => removeTarget(t.id)}>
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <div className="card-title">Add Route</div>
        <p style={{ fontSize: 'var(--text-sm)', color: 'var(--text-secondary)', marginBottom: 'var(--space-3)' }}>
          A route maps a project to a target. When the server sends a delivery instruction
          for an event in a project, the agent uses the route to determine which target to deliver to.
        </p>
        <form onSubmit={addRoute}>
          <div className="form-row">
            <div className="form-field">
              <label>Project ID</label>
              <input
                value={newProjectId}
                onChange={(e) => setNewProjectId(e.target.value)}
                placeholder="550e8400-e29b-41d4-a716-446655440000"
              />
            </div>
            <div className="form-field">
              <label>Target ID</label>
              <select
                value={newTargetId}
                onChange={(e) => setNewTargetId(e.target.value)}
              >
                <option value="">Select target...</option>
                {targets.map((t) => (
                  <option key={t.id} value={t.id}>{t.id}</option>
                ))}
              </select>
            </div>
            <button type="submit" className="btn btn-primary">Add Route</button>
          </div>
        </form>
      </div>

      <div className="card">
        <div className="card-title">Configured Routes</div>
        {routes.length === 0 ? (
          <div className="empty-state">No routes configured</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Project ID</th>
                <th>Target ID</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {routes.map((r) => (
                <tr key={r.project_id}>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>{r.project_id}</td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>{r.target_id}</td>
                  <td>
                    <button className="btn btn-danger btn-sm" onClick={() => removeRoute(r.project_id)}>
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
