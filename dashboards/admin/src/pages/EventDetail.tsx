import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { api, type EventDetail, type Endpoint, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { CopyButton } from '../components/CopyButton';
import { CodeViewer } from '../components/CodeViewer';
import { decodeBase64, formatBytes, formatDateTime } from '../lib/format';
import './EventDetail.css';

export function EventDetail() {
  const { id } = useParams<{ id: string }>();
  const [loading, setLoading] = useState(true);
  const [event, setEvent] = useState<EventDetail | null>(null);
  const [endpoint, setEndpoint] = useState<Endpoint | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [unmasked, setUnmasked] = useState(false);

  useEffect(() => {
    async function load() {
      if (!id) return;
      try {
        const ev = await api.get<EventDetail>(`/api/events/${id}?unmasked=${unmasked}`);
        setEvent(ev);
        if (ev.endpoint_id) {
          try {
            const ep = await api.get<Endpoint>(`/api/endpoints/${ev.endpoint_id}`);
            setEndpoint(ep);
          } catch {
            // endpoint may be deleted
          }
        }
        if (ev.project_id) {
          try {
            const p = await api.get<Project>(`/api/projects/${ev.project_id}`);
            setProject(p);
          } catch {
            // ignore
          }
        }
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [id, unmasked]);

  if (loading) return <Loading />;
  if (!event) return <EmptyState title="Event not found" />;

  const body = event.body ? decodeBase64(event.body) : '';

  return (
    <div className="event-detail">
      <PageHeader
        title="Event"
        subtitle={
          <span className="event-detail-id">
            <code>{event.id}</code>
            <CopyButton value={event.id} />
          </span>
        }
      />

      <div className="event-detail-grid">
        <section className="detail-section">
          <h2 className="detail-section-title">Overview</h2>
          <dl className="kv-list">
            <div className="kv">
              <dt>Method</dt>
              <dd><Badge tone="green">{event.request_method}</Badge></dd>
            </div>
            <div className="kv">
              <dt>Endpoint</dt>
              <dd>
                {endpoint ? (
                  <Link to={`/projects/${event.project_id}`}>{endpoint.name}</Link>
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
              <dd><Badge>{event.delivery_state}</Badge></dd>
            </div>
          </dl>
        </section>

        <section className="detail-section">
          <div className="detail-section-head">
            <h2 className="detail-section-title">Headers</h2>
            <button
              className="event-detail-toggle"
              onClick={() => setUnmasked((u) => !u)}
            >
              {unmasked ? 'Hide secrets' : 'Show secrets'}
            </button>
          </div>
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
          <EmptyState
            title="No deliveries yet"
            description="Connect a local agent and replay this event to deliver it locally."
          />
        </section>
      </div>
    </div>
  );
}
