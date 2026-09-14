import { useEffect, useState, type FormEvent } from 'react';
import { Link, useParams } from 'react-router-dom';
import { api, type Project, type Endpoint, type EventListRow } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { CopyButton } from '../components/CopyButton';
import { useToast } from '../components/Toast';
import { formatBytes, formatRelative } from '../lib/format';
import './ProjectDetail.css';

export function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const [loading, setLoading] = useState(true);
  const [project, setProject] = useState<Project | null>(null);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [events, setEvents] = useState<EventListRow[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState('');
  const [provider, setProvider] = useState('');
  const [creating, setCreating] = useState(false);
  const toast = useToast();

  async function load() {
    if (!id) return;
    try {
      const [p, e, ev] = await Promise.all([
        api.get<Project>(`/api/projects/${id}`),
        api.get<{ endpoints: Endpoint[] }>(`/api/projects/${id}/endpoints`),
        api.get<{ events: EventListRow[] }>(`/api/events?project_id=${id}&limit=10`),
      ]);
      setProject(p);
      setEndpoints(e.endpoints);
      setEvents(ev.events);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  async function handleCreate(e: FormEvent) {
    e.preventDefault();
    if (!id) return;
    setCreating(true);
    try {
      await api.post(`/api/projects/${id}/endpoints`, {
        name,
        provider: provider || null,
      });
      setName('');
      setProvider('');
      setShowCreate(false);
      toast.show('Endpoint created', 'success');
      await load();
    } catch (err) {
      toast.show(err instanceof Error ? err.message : 'Create failed', 'error');
    } finally {
      setCreating(false);
    }
  }

  if (loading) return <Loading />;
  if (!project) return <EmptyState title="Project not found" />;

  const baseUrl = (import.meta.env.VITE_PUBLIC_BASE_URL as string | undefined) ?? '';

  return (
    <div className="project-detail">
      <PageHeader
        title={project.name}
        subtitle={
          <span>
            {project.description || 'No description'}
          </span>
        }
        actions={
          <Button variant="primary" onClick={() => setShowCreate((s) => !s)}>
            New endpoint
          </Button>
        }
      />

      {showCreate ? (
        <form className="card create-form" onSubmit={handleCreate}>
          <Input
            label="Name"
            name="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="paystack"
            autoFocus
            required
          />
          <Input
            label="Provider (optional)"
            name="provider"
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            placeholder="paystack, github, stripe"
            hint="Used for future signature verification."
          />
          <div className="create-form-actions">
            <Button type="submit" variant="primary" loading={creating}>
              Create
            </Button>
            <Button type="button" variant="ghost" onClick={() => setShowCreate(false)}>
              Cancel
            </Button>
          </div>
        </form>
      ) : null}

      <section className="detail-section">
        <h2 className="detail-section-title">Endpoints</h2>
        {endpoints.length === 0 ? (
          <EmptyState
            title="No endpoints yet"
            description="Create an inbound endpoint to receive webhooks for this project."
          />
        ) : (
          <div className="endpoint-list">
            {endpoints.map((ep) => (
              <div key={ep.id} className="endpoint-row">
                <div className="endpoint-row-main">
                  <div className="endpoint-row-head">
                    <span className="endpoint-row-name">{ep.name}</span>
                    {ep.enabled ? <Badge tone="green">Enabled</Badge> : <Badge>Disabled</Badge>}
                    {ep.provider ? <Badge tone="blue">{ep.provider}</Badge> : null}
                  </div>
                  <div className="endpoint-row-url">
                    <code>{baseUrl}/i/{ep.public_identifier}</code>
                    <CopyButton value={`${baseUrl}/i/${ep.public_identifier}`} />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="detail-section">
        <div className="detail-section-head">
          <h2 className="detail-section-title">Recent events</h2>
          {events.length > 0 ? (
            <Link to={`/events?project_id=${project.id}`} className="overview-link">
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
              <Link to={`/events/${ev.id}`} key={ev.id} className="overview-row">
                <Badge tone="green">{ev.request_method}</Badge>
                <span className="overview-row-method">{ev.content_type ?? 'no content type'}</span>
                <span className="overview-row-size">{formatBytes(ev.payload_size)}</span>
                <span className="overview-row-time">{formatRelative(ev.received_at)}</span>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
