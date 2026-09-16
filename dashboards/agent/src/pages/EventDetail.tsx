import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { api, ApiError, type StoredEvent, type Target, type ServerProject, type DeliveryRecord } from '../lib/api';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { CodeViewer } from '../components/CodeViewer';
import { useToast } from '../components/Toast';
import { IconReplay, IconArrowLeft, IconTrash } from '../components/Icons';
import '../components/CodeViewer.css';

export function EventDetail({ id }: { id: string }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<StoredEvent | null>(null);
  const [deliveries, setDeliveries] = useState<DeliveryRecord[]>([]);
  const [targets, setTargets] = useState<Target[]>([]);
  const [endpointMap, setEndpointMap] = useState<Record<string, { name: string; projectName: string }>>({});
  const [replayOpen, setReplayOpen] = useState(false);
  const [replayTarget, setReplayTarget] = useState('');
  const [replaying, setReplaying] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const navigate = useNavigate();
  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [ev, t, p, d] = await Promise.all([
        api.get<StoredEvent>(`/api/events/${id}`),
        api.get<{ targets: Target[] }>('/api/targets'),
        api.get<{ projects: ServerProject[] }>('/api/projects'),
        api.get<{ deliveries: DeliveryRecord[] }>(
          `/api/events/${id}/deliveries`,
        ).catch(() => ({ deliveries: [] as DeliveryRecord[] })),
      ]);
      setEvent(ev);
      setTargets(t.targets);
      setDeliveries(d.deliveries);
      const map: Record<string, { name: string; projectName: string }> = {};
      for (const proj of p.projects ?? []) {
        for (const e of proj.endpoints ?? []) {
          map[e.id] = { name: e.name, projectName: proj.name };
        }
      }
      setEndpointMap(map);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load event');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  const submitReplay = async () => {
    if (!event || !replayTarget.trim()) return;
    setReplaying(true);
    try {
      await api.post(
        `/api/events/${encodeURIComponent(event.id)}/replay`,
        { target_id: replayTarget.trim() },
      );
      toast.show('Replay dispatched. Check History for the result.', 'success');
      setReplayOpen(false);
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Replay failed',
        'error',
      );
    } finally {
      setReplaying(false);
    }
  };

  const deleteEvent = async () => {
    if (!event) return;
    setDeleting(true);
    try {
      await api.delete(`/api/events/${encodeURIComponent(event.id)}`);
      toast.show('Event deleted locally', 'success');
      navigate('/events');
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to delete event',
        'error',
      );
      setDeleting(false);
      setDeleteOpen(false);
    }
  };

  if (loading) {
    return (
      <div>
        <div className="page-header">
          <div className="page-header-row">
            <div>
              <h1>Event</h1>
              <p>Loading event details.</p>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="empty-state">Loading...</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <div className="page-header">
          <div className="page-header-row">
            <div>
              <h1>Event</h1>
              <p>Could not load this event.</p>
            </div>
          </div>
        </div>
        <div className="card error-card">
          <div className="error-title">Unable to load event</div>
          <div className="error-detail">{error}</div>
          <Link to="/events" className="btn btn-sm">Back to events</Link>
        </div>
      </div>
    );
  }

  if (!event) {
    return (
      <div>
        <div className="page-header">
          <div className="page-header-row">
            <div>
              <h1>Event</h1>
              <p>Event not found in local storage.</p>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="empty-state">Event not found.</div>
        </div>
      </div>
    );
  }

  const projectName = event.project_id
    ? (endpointMap[event.endpoint_id]?.projectName ?? event.project_id)
    : '-';
  const endpointName = event.endpoint_id
    ? (endpointMap[event.endpoint_id]?.name ?? event.endpoint_id)
    : '-';

  const decodedBody = event.body ? atob(event.body) : '';

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <div className="page-header-back">
              <Link to="/events" className="page-header-back-link">
                <IconArrowLeft size={14} /> Events
              </Link>
            </div>
            <h1>Event detail</h1>
            <p>Locally stored webhook event.</p>
          </div>
          <div className="page-header-actions">
            <button
              className="btn btn-danger btn-sm"
              onClick={() => setDeleteOpen(true)}
            >
              <IconTrash size={14} /> Delete
            </button>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => {
                setReplayTarget(targets.length === 1 ? targets[0].id : '');
                setReplayOpen(true);
              }}
            >
              <IconReplay size={14} /> Replay
            </button>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Overview</div>
        <div className="status-row">
          <span className="status-label">Event ID</span>
          <span className="status-value mono">{event.id}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Project</span>
          <span className="status-value">{projectName}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Endpoint</span>
          <span className="status-value">{endpointName}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Method</span>
          <span className="status-value">
            <span className="badge badge-pending">{event.request_method}</span>
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Content type</span>
          <span className="status-value">{event.content_type ?? 'none'}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Received</span>
          <span className="status-value mono">{event.received_at}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Size</span>
          <span className="status-value">{event.payload_size} bytes</span>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Headers</div>
        {Object.keys(event.headers).length === 0 ? (
          <div className="empty-state">No headers captured.</div>
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Value</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(event.headers).map(([k, v]) => (
                  <tr key={k}>
                    <td className="mono text-secondary">{k}</td>
                    <td className="mono">
                      {Array.isArray(v) ? v.join(', ') : v}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="card">
        <div className="card-title">Payload</div>
        {event.body ? (
          <CodeViewer body={decodedBody} contentType={event.content_type} />
        ) : (
          <div className="empty-state">No body captured.</div>
        )}
      </div>

      <div className="card">
        <div className="card-title">Deliveries</div>
        {deliveries.length === 0 ? (
          <div className="empty-state">
            No delivery attempts for this event yet. Replay it to deliver it
            locally.
          </div>
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Status</th>
                  <th scope="col">HTTP</th>
                  <th scope="col">Duration</th>
                  <th scope="col">Target</th>
                  <th scope="col">Started</th>
                  <th scope="col">Error</th>
                </tr>
              </thead>
              <tbody>
                {deliveries.map((r) => (
                  <tr
                    key={r.id}
                    className="clickable-row"
                    onClick={() => navigate(`/deliveries/${r.id}`)}
                  >
                    <td>
                      {r.status === 'delivered' ? (
                        <span className="badge badge-success">{r.status}</span>
                      ) : r.status === 'failed' ? (
                        <span className="badge badge-error">{r.status}</span>
                      ) : (
                        <span className="badge badge-pending">{r.status}</span>
                      )}
                    </td>
                    <td className="mono">{r.http_status ?? '-'}</td>
                    <td className="mono">
                      {r.duration_ms != null ? `${r.duration_ms}ms` : '-'}
                    </td>
                    <td className="mono">{r.target_id || '-'}</td>
                    <td className="mono">{r.started_at}</td>
                    <td className="error-cell">{r.error_message ?? '-'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <Dialog
        open={replayOpen}
        title="Replay event locally"
        description="Deliver this event to a configured local target. The original event is not modified."
        onClose={() => setReplayOpen(false)}
        footer={
          <>
            <button
              type="button"
              className="btn btn-sm"
              onClick={() => setReplayOpen(false)}
              disabled={replaying}
            >
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={submitReplay}
              disabled={replaying || !replayTarget.trim()}
            >
              {replaying ? 'Dispatching...' : 'Dispatch replay'}
            </button>
          </>
        }
      >
        {targets.length === 0 ? (
          <div className="empty-state">
            No targets configured. Add a target on the Targets page before
            replaying.
          </div>
        ) : (
          <div className="login-field">
            <label htmlFor="replay-target">Target</label>
            <select
              id="replay-target"
              value={replayTarget}
              onChange={(e) => setReplayTarget(e.target.value)}
              autoFocus
            >
              <option value="">Select a target</option>
              {targets.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.id}
                </option>
              ))}
            </select>
          </div>
        )}
      </Dialog>

      <ConfirmDialog
        open={deleteOpen}
        title="Delete event"
        description="Remove this event from local storage? Delivery records are kept. The event on the server is not affected."
        confirmLabel="Delete event"
        destructive
        loading={deleting}
        onConfirm={deleteEvent}
        onCancel={() => setDeleteOpen(false)}
      />
    </div>
  );
}
