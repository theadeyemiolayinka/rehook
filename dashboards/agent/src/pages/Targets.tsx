import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type Target, type RouteEntry } from '../lib/api';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconPlus, IconTrash, IconRefresh, IconRoute, IconEdit } from '../components/Icons';

export function Targets() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [targets, setTargets] = useState<Target[]>([]);
  const [routes, setRoutes] = useState<RouteEntry[]>([]);

  const [addOpen, setAddOpen] = useState(false);
  const [newId, setNewId] = useState('');
  const [newUrl, setNewUrl] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const [removeTarget, setRemoveTarget] = useState<Target | null>(null);
  const [removing, setRemoving] = useState(false);

  const [editTarget, setEditTarget] = useState<Target | null>(null);
  const [editUrl, setEditUrl] = useState('');
  const [editing, setEditing] = useState(false);

  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [targetsResp, routesResp] = await Promise.all([
        api.get<{ targets: Target[] }>('/api/targets'),
        api.get<{ routes: RouteEntry[] }>('/api/routes'),
      ]);
      setTargets(targetsResp.targets);
      setRoutes(routesResp.routes);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const addTarget = async () => {
    if (!newId.trim() || !newUrl.trim()) return;
    setSubmitting(true);
    try {
      await api.post('/api/targets', { id: newId.trim(), url: newUrl.trim() });
      setNewId('');
      setNewUrl('');
      setAddOpen(false);
      toast.show('Target added', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to add target',
        'error',
      );
    } finally {
      setSubmitting(false);
    }
  };

  const confirmRemove = async () => {
    if (!removeTarget) return;
    setRemoving(true);
    try {
      await api.delete(`/api/targets/${encodeURIComponent(removeTarget.id)}`);
      toast.show('Target removed', 'success');
      setRemoveTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to remove target',
        'error',
      );
    } finally {
      setRemoving(false);
    }
  };

  const openEdit = (t: Target) => {
    setEditTarget(t);
    setEditUrl(t.url);
  };

  const saveEdit = async () => {
    if (!editTarget || !editUrl.trim()) return;
    setEditing(true);
    try {
      await api.patch(`/api/targets/${encodeURIComponent(editTarget.id)}`, {
        url: editUrl.trim(),
      });
      toast.show('Target updated', 'success');
      setEditTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to update target',
        'error',
      );
    } finally {
      setEditing(false);
    }
  };

  const routeCountFor = (targetId: string) =>
    routes.filter((r) => r.target_id === targetId).length;

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <h1>Targets</h1>
            <p>
              Local HTTP destinations the agent can deliver webhooks to. Only
              http and https URLs are allowed. The agent never accepts URLs
              from the server; it only resolves target IDs you configure here.
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
            >
              <IconPlus size={14} /> New target
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

      <div className="card">
        {loading ? (
          <div className="empty-state">Loading...</div>
        ) : targets.length === 0 ? (
          <div className="empty-state">
            <div className="empty-title">No targets configured</div>
            <div className="empty-detail">
              A target is a local HTTP endpoint (like
              <span className="mono"> http://localhost:8000/webhook</span>)
              that will receive delivered webhooks. Add one to get started.
            </div>
          </div>
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">ID</th>
                  <th scope="col">URL</th>
                  <th scope="col">Routes</th>
                  <th scope="col"></th>
                </tr>
              </thead>
              <tbody>
                {targets.map((t) => {
                  const count = routeCountFor(t.id);
                  return (
                    <tr key={t.id}>
                      <td className="mono">{t.id}</td>
                      <td className="mono">{t.url}</td>
                      <td>
                        {count > 0 ? (
                          <Link to="/routes" className="link-badge">
                            <IconRoute size={12} /> {count} route{count > 1 ? 's' : ''}
                          </Link>
                        ) : (
                          <span className="text-tertiary">none</span>
                        )}
                      </td>
                      <td className="row-action">
                        <button
                          className="icon-btn"
                          onClick={() => openEdit(t)}
                          aria-label={`Edit target ${t.id}`}
                        >
                          <IconEdit size={14} />
                        </button>
                        <button
                          className="icon-btn"
                          onClick={() => setRemoveTarget(t)}
                          aria-label={`Remove target ${t.id}`}
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
        title="New target"
        description="A local HTTP destination the agent can deliver webhooks to. Only http and https URLs are allowed."
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
              onClick={addTarget}
              disabled={submitting || !newId.trim() || !newUrl.trim()}
            >
              {submitting ? 'Adding...' : 'Add target'}
            </button>
          </>
        }
      >
        <div className="login-field">
          <label htmlFor="target-id">Target ID</label>
          <input
            id="target-id"
            value={newId}
            onChange={(e) => setNewId(e.target.value)}
            placeholder="myapp"
            autoFocus
          />
          <span className="field-hint">
            A short name you choose. Routes reference targets by this ID.
          </span>
        </div>
        <div className="login-field">
          <label htmlFor="target-url">Local URL</label>
          <input
            id="target-url"
            value={newUrl}
            onChange={(e) => setNewUrl(e.target.value)}
            placeholder="http://localhost:8000/webhook"
          />
          <span className="field-hint">
            The local application endpoint that will receive delivered
            webhooks.
          </span>
        </div>
      </Dialog>

      <Dialog
        open={!!editTarget}
        title={`Edit target "${editTarget?.id ?? ''}"`}
        description="Update the local URL for this target. Routes referencing it keep working; new deliveries use the new URL."
        onClose={() => setEditTarget(null)}
        footer={
          <>
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => setEditTarget(null)}
              disabled={editing}
            >
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={saveEdit}
              disabled={editing || !editUrl.trim()}
            >
              {editing ? 'Saving...' : 'Save changes'}
            </button>
          </>
        }
      >
        <div className="login-field">
          <label htmlFor="edit-target-url">Local URL</label>
          <input
            id="edit-target-url"
            value={editUrl}
            onChange={(e) => setEditUrl(e.target.value)}
            placeholder="http://localhost:8000/webhook"
            autoFocus
          />
          <span className="field-hint">
            The local application endpoint that will receive delivered
            webhooks. Only http and https URLs are allowed.
          </span>
        </div>
      </Dialog>

      <ConfirmDialog
        open={!!removeTarget}
        title="Remove target"
        description={
          removeTarget
            ? `Remove target "${removeTarget.id}"? Routes using this target will also be removed.`
            : ''
        }
        confirmLabel="Remove target"
        destructive
        loading={removing}
        onConfirm={confirmRemove}
        onCancel={() => setRemoveTarget(null)}
      />
    </div>
  );
}
