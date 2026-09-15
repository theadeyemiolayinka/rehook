import { useCallback, useEffect, useState } from 'react';
import { api, ApiError, type RouteEntry, type ServerProject, type ServerEndpoint, type Target } from '../lib/api';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconPlus, IconTrash, IconRefresh, IconWarning } from '../components/Icons';

export function Routes() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [routes, setRoutes] = useState<RouteEntry[]>([]);
  const [targets, setTargets] = useState<Target[]>([]);
  const [projects, setProjects] = useState<ServerProject[]>([]);
  const [endpointMap, setEndpointMap] = useState<Record<string, ServerEndpoint>>({});

  const [addOpen, setAddOpen] = useState(false);
  const [newEndpointId, setNewEndpointId] = useState('');
  const [newTargetId, setNewTargetId] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const [removeRoute, setRemoveRoute] = useState<RouteEntry | null>(null);
  const [removing, setRemoving] = useState(false);

  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [routesResp, targetsResp, projectsResp] = await Promise.all([
        api.get<{ routes: RouteEntry[] }>('/api/routes'),
        api.get<{ targets: Target[] }>('/api/targets'),
        api.get<{ projects: ServerProject[] }>('/api/projects'),
      ]);
      setRoutes(routesResp.routes);
      setTargets(targetsResp.targets);
      setProjects(projectsResp.projects ?? []);
      const map: Record<string, ServerEndpoint> = {};
      for (const p of projectsResp.projects ?? []) {
        for (const e of p.endpoints ?? []) {
          map[e.id] = e;
        }
      }
      setEndpointMap(map);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load routes');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const addRoute = async () => {
    if (!newEndpointId.trim() || !newTargetId.trim()) return;
    setSubmitting(true);
    try {
      await api.post('/api/routes', {
        endpoint_id: newEndpointId.trim(),
        target_id: newTargetId.trim(),
      });
      setNewEndpointId('');
      setNewTargetId('');
      setAddOpen(false);
      toast.show('Route added', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to add route',
        'error',
      );
    } finally {
      setSubmitting(false);
    }
  };

  const confirmRemove = async () => {
    if (!removeRoute) return;
    setRemoving(true);
    try {
      await api.delete(`/api/routes/${encodeURIComponent(removeRoute.endpoint_id)}`);
      toast.show('Route removed', 'success');
      setRemoveRoute(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to remove route',
        'error',
      );
    } finally {
      setRemoving(false);
    }
  };

  const allEndpoints: { id: string; label: string; projectName: string }[] = [];
  for (const p of projects) {
    for (const e of p.endpoints ?? []) {
      allEndpoints.push({ id: e.id, label: e.name, projectName: p.name });
    }
  }

  const staleRoutes = routes.filter((r) => !endpointMap[r.endpoint_id]);

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <h1>Routes</h1>
            <p>
              Connect inbound webhook endpoints to local targets. When the
              server sends a delivery for an event on an endpoint, the agent
              uses the route to find the right local target.
            </p>
          </div>
          <div className="page-header-actions">
            <button type="button" className="btn btn-sm" onClick={load}>
              <IconRefresh size={14} /> Refresh
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={() => setAddOpen(true)}
              disabled={targets.length === 0}
            >
              <IconPlus size={14} /> New route
            </button>
          </div>
        </div>
      </div>

      {error ? (
        <div className="card error-card">
          <div className="error-title">Unable to load</div>
          <div className="error-detail">{error}</div>
          <button className="btn btn-sm" onClick={load}>
            Retry
          </button>
        </div>
      ) : null}

      {staleRoutes.length > 0 ? (
        <div className="card warning-card">
          <div className="warning-title">
            <IconWarning size={14} /> Stale routes detected
          </div>
          <div className="warning-detail">
            {staleRoutes.length} route(s) reference endpoints that no longer
            exist on the server. These routes will never receive deliveries.
            Remove them to clean up.
          </div>
        </div>
      ) : null}

      <div className="card">
        {loading ? (
          <div className="empty-state">Loading...</div>
        ) : routes.length === 0 ? (
          <div className="empty-state">
            <div className="empty-title">No routes configured</div>
            <div className="empty-detail">
              A route tells the agent which local target should receive events
              from a specific endpoint. Create one to start delivering
              webhooks to your local application.
            </div>
          </div>
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Endpoint</th>
                  <th scope="col">Project</th>
                  <th scope="col">Target</th>
                  <th scope="col"></th>
                </tr>
              </thead>
              <tbody>
                {routes.map((r) => {
                  const ep = endpointMap[r.endpoint_id];
                  const target = targets.find((t) => t.id === r.target_id);
                  return (
                    <tr key={r.endpoint_id}>
                      <td>
                        {ep ? (
                          <span>{ep.name}</span>
                        ) : (
                          <span className="text-tertiary mono stale-id">
                            <IconWarning size={12} /> {r.endpoint_id.slice(0, 8)}
                          </span>
                        )}
                      </td>
                      <td className="text-secondary">
                        {ep
                          ? projects.find((p) =>
                              (p.endpoints ?? []).some((e) => e.id === r.endpoint_id),
                            )?.name ?? '-'
                          : '-'}
                      </td>
                      <td>
                        {target ? (
                          <span className="mono">{target.id}</span>
                        ) : (
                          <span className="text-tertiary mono">
                            {r.target_id}
                          </span>
                        )}
                      </td>
                      <td className="row-action">
                        <button
                          className="icon-btn"
                          onClick={() => setRemoveRoute(r)}
                          aria-label="Remove route"
                        >
                          <IconTrash size={14} />
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <Dialog
        open={addOpen}
        title="New route"
        description="Pick an endpoint from your HookRelay server and connect it to a local target. Events received on that endpoint will be delivered to the target."
        onClose={() => setAddOpen(false)}
        footer={
          <>
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => setAddOpen(false)}
              disabled={submitting}
            >
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={addRoute}
              disabled={submitting || !newEndpointId.trim() || !newTargetId.trim()}
            >
              {submitting ? 'Adding...' : 'Add route'}
            </button>
          </>
        }
      >
        {allEndpoints.length === 0 ? (
          <div className="empty-state">
            No endpoints available from the server. Make sure the agent is
            connected and the server has projects with endpoints configured.
          </div>
        ) : (
          <>
            <div className="login-field">
              <label htmlFor="route-endpoint">Endpoint</label>
              <select
                id="route-endpoint"
                value={newEndpointId}
                onChange={(e) => setNewEndpointId(e.target.value)}
                autoFocus
              >
                <option value="">Select an endpoint</option>
                {projects.map((p) => (
                  <optgroup key={p.id} label={p.name}>
                    {(p.endpoints ?? []).map((e) => (
                      <option key={e.id} value={e.id}>
                        {e.name}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <span className="field-hint">
                The inbound webhook endpoint whose events should be delivered
                locally.
              </span>
            </div>
            <div className="login-field">
              <label htmlFor="route-target">Target</label>
              <select
                id="route-target"
                value={newTargetId}
                onChange={(e) => setNewTargetId(e.target.value)}
              >
                <option value="">Select a target</option>
                {targets.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.id} ({t.url})
                  </option>
                ))}
              </select>
              <span className="field-hint">
                The local application that will receive the delivered events.
              </span>
            </div>
          </>
        )}
      </Dialog>

      <ConfirmDialog
        open={!!removeRoute}
        title="Remove route"
        description={
          removeRoute
            ? `Remove the route for endpoint "${endpointMap[removeRoute.endpoint_id]?.name ?? removeRoute.endpoint_id.slice(0, 8)}"? Events from this endpoint will no longer be delivered locally.`
            : ''
        }
        confirmLabel="Remove route"
        destructive
        loading={removing}
        onConfirm={confirmRemove}
        onCancel={() => setRemoveRoute(null)}
      />
    </div>
  );
}
