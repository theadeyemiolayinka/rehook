import { useCallback, useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { api, ApiError, type Project, type Endpoint, type EventListRow } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { CopyButton } from '../components/CopyButton';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconPlus, IconTrash } from '../components/Icons';
import { formatBytes, formatRelative } from '../lib/format';
import './ProjectDetail.css';

export function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [events, setEvents] = useState<EventListRow[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [provider, setProvider] = useState('');
  const [creating, setCreating] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Endpoint | null>(null);
  const [deleting, setDeleting] = useState(false);
  const toast = useToast();

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    setError(null);
    try {
      const [p, e, ev] = await Promise.all([
        api.get<Project>(`/api/projects/${id}`),
        api.get<{ endpoints: Endpoint[] }>(`/api/projects/${id}/endpoints`),
        api.get<{ events: EventListRow[] }>(
          `/api/events?project_id=${id}&limit=10`,
        ),
      ]);
      setProject(p);
      setEndpoints(e.endpoints);
      setEvents(ev.events);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load project');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  async function handleCreate() {
    if (!id) return;
    setCreating(true);
    try {
      await api.post(`/api/projects/${id}/endpoints`, {
        name: name.trim(),
        provider: provider.trim() || null,
      });
      setName('');
      setProvider('');
      setCreateOpen(false);
      toast.show('Endpoint created', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Create failed',
        'error',
      );
    } finally {
      setCreating(false);
    }
  }

  async function handleDelete() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.delete(`/api/endpoints/${deleteTarget.id}`);
      toast.show('Endpoint deleted', 'success');
      setDeleteTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Delete failed',
        'error',
      );
    } finally {
      setDeleting(false);
    }
  }

  if (loading) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <div className="project-detail-loading">
          <Loading />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <EmptyState
          title="Unable to load project"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      </div>
    );
  }

  if (!project) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <EmptyState title="Project not found" />
      </div>
    );
  }

  const baseUrl = (import.meta.env.VITE_PUBLIC_BASE_URL as string | undefined) ?? '';

  return (
    <div className="project-detail">
      <PageHeader
        title={project.name}
        subtitle={project.description || 'No description'}
        actions={
          <Button variant="primary" size="sm" onClick={() => setCreateOpen(true)}>
            <IconPlus size={14} /> New endpoint
          </Button>
        }
      />

      <section className="detail-section">
        <h2 className="detail-section-title">Endpoints</h2>
        {endpoints.length === 0 ? (
          <EmptyState
            title="No endpoints yet"
            description="Create an inbound endpoint to receive webhooks for this project."
            action={
              <Button variant="primary" onClick={() => setCreateOpen(true)}>
                <IconPlus size={14} /> New endpoint
              </Button>
            }
          />
        ) : (
          <div className="endpoint-list">
            {endpoints.map((ep) => (
              <div key={ep.id} className="endpoint-row">
                <div className="endpoint-row-main">
                  <div className="endpoint-row-head">
                    <span className="endpoint-row-name">{ep.name}</span>
                    {ep.enabled ? (
                      <Badge tone="green">Enabled</Badge>
                    ) : (
                      <Badge>Disabled</Badge>
                    )}
                    {ep.provider ? (
                      <Badge tone="blue">{ep.provider}</Badge>
                    ) : null}
                  </div>
                  <div className="endpoint-row-url">
                    <code>
                      {baseUrl}/i/{ep.public_identifier}
                    </code>
                    <CopyButton
                      value={`${baseUrl}/i/${ep.public_identifier}`}
                      compact
                    />
                  </div>
                </div>
                <button
                  type="button"
                  className="endpoint-row-delete"
                  onClick={() => setDeleteTarget(ep)}
                  aria-label={`Delete endpoint ${ep.name}`}
                >
                  <IconTrash size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="detail-section">
        <div className="detail-section-head">
          <h2 className="detail-section-title">Recent events</h2>
          {events.length > 0 ? (
            <Link
              to={`/events?project_id=${project.id}`}
              className="overview-link"
            >
              View all
            </Link>
          ) : null}
        </div>
        {events.length === 0 ? (
          <EmptyState
            title="No events yet"
            description="Send a request to an endpoint above to capture a webhook."
          />
        ) : (
          <div className="overview-list">
            {events.map((ev) => (
              <Link
                to={`/events/${ev.id}`}
                key={ev.id}
                className="overview-row"
              >
                <Badge tone="green">{ev.request_method}</Badge>
                <span className="overview-row-method">
                  {ev.content_type ?? 'no content type'}
                </span>
                <span className="overview-row-size">
                  {formatBytes(ev.payload_size)}
                </span>
                <span className="overview-row-time">
                  {formatRelative(ev.received_at)}
                </span>
              </Link>
            ))}
          </div>
        )}
      </section>

      <Dialog
        open={createOpen}
        title="New endpoint"
        description="An inbound endpoint receives webhooks at a unique URL."
        onClose={() => setCreateOpen(false)}
        footer={
          <>
            <Button
              variant="ghost"
              onClick={() => setCreateOpen(false)}
              disabled={creating}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleCreate}
              loading={creating}
              disabled={!name.trim()}
            >
              Create endpoint
            </Button>
          </>
        }
      >
        <Input
          label="Name"
          name="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="paystack"
          autoFocus
          required
        />
        {/* <Input
          label="Provider (optional)"
          name="provider"
          value={provider}
          onChange={(e) => setProvider(e.target.value)}
          placeholder="paystack, github, stripe"
          hint="Used for future signature verification."
        /> */}
      </Dialog>

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete endpoint"
        description={
          deleteTarget
            ? `Delete ${deleteTarget.name}? All captured events for this endpoint will also be deleted. This cannot be undone.`
            : ''
        }
        confirmLabel="Delete endpoint"
        destructive
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}
