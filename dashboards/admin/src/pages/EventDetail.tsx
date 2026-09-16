import { useCallback, useEffect, useState } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import {
  api,
  ApiError,
  type EventDetail,
  type Endpoint,
  type Project,
  type Agent,
  type DeliveryRow,
} from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { Button } from '../components/Button';
import { CopyButton } from '../components/CopyButton';
import { CodeViewer } from '../components/CodeViewer';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { StatusIndicator } from '../components/StatusIndicator';
import { useToast } from '../components/Toast';
import { IconReplay, IconEye, IconEyeOff, IconTrash } from '../components/Icons';
import { decodeBase64, formatBytes, formatDateTime, formatRelative } from '../lib/format';
import './EventDetail.css';

export function EventDetail() {
  const { id } = useParams<{ id: string }>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<EventDetail | null>(null);
  const [endpoint, setEndpoint] = useState<Endpoint | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [deliveries, setDeliveries] = useState<DeliveryRow[]>([]);
  const [unmasked, setUnmasked] = useState(false);
  const [replayOpen, setReplayOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const navigate = useNavigate();
  const toast = useToast();

  const load = useCallback(async (showSpinner = true, unmaskedOverride?: boolean) => {
    if (!id) return;
    const useUnmasked = unmaskedOverride !== undefined ? unmaskedOverride : unmasked;
    if (showSpinner) setLoading(true);
    setError(null);
    try {
      const ev = await api.get<EventDetail>(
        `/api/events/${id}?unmasked=${useUnmasked}`,
      );
      setEvent(ev);
      if (ev.endpoint_id) {
        try {
          const ep = await api.get<Endpoint>(`/api/endpoints/${ev.endpoint_id}`);
          setEndpoint(ep);
        } catch {
          setEndpoint(null);
        }
      }
      if (ev.project_id) {
        try {
          const p = await api.get<Project>(`/api/projects/${ev.project_id}`);
          setProject(p);
        } catch {
          setProject(null);
        }
      }
      try {
        const d = await api.get<{ deliveries: DeliveryRow[] }>(
          `/api/events/${id}/deliveries`,
        );
        setDeliveries(d.deliveries);
      } catch {
        setDeliveries([]);
      }
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load event');
    } finally {
      if (showSpinner) setLoading(false);
    }
  }, [id, unmasked]);

  useEffect(() => {
    load();
  }, [id]); // Only run on mount and when id changes

  const handleDelete = async () => {
    if (!id) return;
    setDeleting(true);
    try {
      await api.delete(`/api/events/${id}`);
      toast.show('Event deleted', 'success');
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

  const toggleUnmasked = () => {
    const next = !unmasked;
    setUnmasked(next);
    // Reload with the new unmasked state, without showing the loading spinner.
    load(false, next);
  };

  if (loading) {
    return (
      <div className="event-detail">
        <PageHeader title="Event" />
        <div className="event-detail-loading">
          <Loading />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="event-detail">
        <PageHeader title="Event" />
        <EmptyState
          title="Unable to load event"
          description={error}
          action={<Button onClick={() => load()}>Retry</Button>}
        />
      </div>
    );
  }

  if (!event) {
    return (
      <div className="event-detail">
        <PageHeader title="Event" />
        <EmptyState title="Event not found" />
      </div>
    );
  }

  const body = event.body ? decodeBase64(event.body) : '';

  return (
    <div className="event-detail">
      <PageHeader
        title="Event"
        subtitle={
          <span className="event-detail-id">
            <code>{event.id}</code>
            <CopyButton value={event.id} compact />
          </span>
        }
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setDeleteOpen(true)}
              aria-label="Delete event"
            >
              <IconTrash size={14} />
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => setReplayOpen(true)}
            >
              <IconReplay size={14} /> Replay
            </Button>
          </>
        }
      />

      <div className="event-detail-grid">
        <section className="detail-section">
          <h2 className="detail-section-title">Overview</h2>
          <dl className="kv-list">
            <div className="kv">
              <dt>Method</dt>
              <dd>
                <Badge tone="green">{event.request_method}</Badge>
              </dd>
            </div>
            <div className="kv">
              <dt>Endpoint</dt>
              <dd>
                {endpoint ? (
                  <Link to={`/projects/${event.project_id}`}>
                    {endpoint.name}
                  </Link>
                ) : (
                  <span className="text-tertiary">deleted</span>
                )}
              </dd>
            </div>
            <div className="kv">
              <dt>Project</dt>
              <dd>
                {project ? (
                  <Link to={`/projects/${event.project_id}`}>{project.name}</Link>
                ) : (
                  <span className="text-tertiary">deleted</span>
                )}
              </dd>
            </div>
            <div className="kv">
              <dt>Content type</dt>
              <dd className="kv-mono">{event.content_type ?? '-'}</dd>
            </div>
            <div className="kv">
              <dt>Remote address</dt>
              <dd className="kv-mono">{event.remote_address ?? '-'}</dd>
            </div>
            <div className="kv">
              <dt>Received</dt>
              <dd>{formatDateTime(event.received_at)}</dd>
            </div>
            <div className="kv">
              <dt>Payload size</dt>
              <dd>{formatBytes(event.payload_size)}</dd>
            </div>
            <div className="kv">
              <dt>Delivery state</dt>
              <dd>
                <Badge>{event.delivery_state}</Badge>
              </dd>
            </div>
          </dl>
        </section>

        <section className="detail-section">
          <div className="detail-section-head">
            <h2 className="detail-section-title">Headers</h2>
            <button
              type="button"
              className="event-detail-toggle"
              onClick={toggleUnmasked}
              aria-pressed={unmasked}
            >
              {unmasked ? (
                <>
                  <IconEyeOff size={13} /> Hide secrets
                </>
              ) : (
                <>
                  <IconEye size={13} /> Show secrets
                </>
              )}
            </button>
          </div>
          {Object.keys(event.headers).length === 0 ? (
            <EmptyState title="No headers" />
          ) : (
            <div className="headers-list">
              {Object.entries(event.headers).map(([k, v]) => (
                <div className="header-row" key={k}>
                  <div className="header-name">{k}</div>
                  <div className="header-value">
                    {Array.isArray(v) ? v.join(', ') : v}
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="detail-section">
          <h2 className="detail-section-title">Payload</h2>
          {event.body ? (
            <CodeViewer body={body} contentType={event.content_type} />
          ) : (
            <EmptyState title="No body" description="This request had no body." />
          )}
        </section>

        <section className="detail-section">
          <h2 className="detail-section-title">Deliveries</h2>
          {deliveries.length === 0 ? (
            <EmptyState
              title="No deliveries yet"
              description="Replay this event to deliver it to a connected agent."
            />
          ) : (
            <div className="deliveries-table-wrap">
              <table className="deliveries-table">
                <thead>
                  <tr>
                    <th scope="col">#</th>
                    <th scope="col">Status</th>
                    <th scope="col">Agent</th>
                    <th scope="col">HTTP</th>
                    <th scope="col">Duration</th>
                    <th scope="col">Target</th>
                    <th scope="col">Started</th>
                    <th scope="col">Error</th>
                  </tr>
                </thead>
                <tbody>
                  {deliveries.map((d) => (
                    <tr key={d.id}>
                      <td className="deliv-num">{d.attempt_number}</td>
                      <td>
                        <StatusIndicator
                          state={deliveryState(d.status)}
                          label={d.status}
                        />
                      </td>
                      <td>
                        {d.agent_name ?? (
                          <span className="text-tertiary">unknown</span>
                        )}
                      </td>
                      <td className="deliv-mono">
                        {d.http_status ?? '-'}
                      </td>
                      <td className="deliv-mono">
                        {d.duration_ms != null ? `${d.duration_ms}ms` : '-'}
                      </td>
                      <td className="deliv-mono deliv-target">
                        {d.target_id || '-'}
                      </td>
                      <td className="deliv-mono">
                        {formatRelative(d.started_at)}
                      </td>
                      <td className="deliv-error">
                        {d.error_message ?? '-'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>

      <ReplayDialog
        open={replayOpen}
        eventId={event.id}
        projectName={project?.name}
        onClose={() => setReplayOpen(false)}
        onDone={load}
      />

      <ConfirmDialog
        open={deleteOpen}
        title="Delete event"
        description="Permanently delete this event and its delivery records? This cannot be undone."
        confirmLabel="Delete event"
        destructive
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setDeleteOpen(false)}
      />
    </div>
  );
}

function deliveryState(
  status: string,
): 'connected' | 'failed' | 'pending' | 'delivered' {
  if (status === 'delivered') return 'delivered';
  if (status === 'failed') return 'failed';
  return 'pending';
}

interface ReplayProps {
  open: boolean;
  eventId: string;
  projectName?: string;
  onClose: () => void;
  onDone: () => void;
}

function ReplayDialog({ open, eventId, onClose, onDone }: ReplayProps) {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [agentId, setAgentId] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const toast = useToast();

  useEffect(() => {
    if (!open) return;
    setAgentId('');
    api
      .get<{ agents: Agent[] }>('/api/agents')
      .then((res) => {
        const connected = res.agents.filter((a) => a.enabled && a.connected);
        setAgents(connected);
        if (connected.length === 1) setAgentId(connected[0].id);
      })
      .catch(() => setAgents([]));
  }, [open]);

  async function submit() {
    if (!agentId) return;
    setSubmitting(true);
    try {
      await api.post(`/api/events/${eventId}/replay`, {
        agent_id: agentId,
      });
      toast.show('Replay dispatched', 'success');
      onClose();
      onDone();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Replay failed',
        'error',
      );
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open={open}
      title="Replay event"
      description="Send this event to a connected agent for local delivery. The agent resolves the destination from its own routes; the server never sends a target address. The original event is not modified."
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={submitting}>
            Cancel
          </Button>
          <Button
            variant="primary"
            onClick={submit}
            loading={submitting}
            disabled={!agentId}
          >
            Dispatch replay
          </Button>
        </>
      }
    >
      {agents.length === 0 ? (
        <div className="replay-no-agents">
          No connected agents. Start the agent on your machine and ensure it is
          connected.
        </div>
      ) : (
        <div className="field">
          <label className="field-label" htmlFor="replay-agent">
            Agent
          </label>
          <select
            id="replay-agent"
            className="input"
            value={agentId}
            onChange={(e) => setAgentId(e.target.value)}
            autoFocus
          >
            <option value="">Select an agent</option>
            {agents.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <span className="field-hint">
            The agent must have a route configured for this endpoint, or the
            replay is dropped.
          </span>
        </div>
      )}
    </Dialog>
  );
}
