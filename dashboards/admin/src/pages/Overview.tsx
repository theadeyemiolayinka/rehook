import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, type EventListRow, type Project, type Endpoint } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { formatBytes, formatRelative } from '../lib/format';
import './Overview.css';

export function Overview() {
  const [loading, setLoading] = useState(true);
  const [projects, setProjects] = useState<Project[]>([]);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [events, setEvents] = useState<EventListRow[]>([]);

  useEffect(() => {
    async function load() {
      try {
        const [p, e, ev] = await Promise.all([
          api.get<{ projects: Project[] }>('/api/projects'),
          api.get<{ endpoints: Endpoint[] }>('/api/endpoints'),
          api.get<{ events: EventListRow[] }>('/api/events?limit=10'),
        ]);
        setProjects(p.projects);
        setEndpoints(e.endpoints);
        setEvents(ev.events);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (loading) return <Loading />;

  return (
    <div className="overview">
      <PageHeader title="Overview" subtitle="Recent activity across your webhook projects." />

      <div className="overview-stats">
        <div className="stat">
          <div className="stat-value">{projects.length}</div>
          <div className="stat-label">Projects</div>
        </div>
        <div className="stat">
          <div className="stat-value">{endpoints.length}</div>
          <div className="stat-label">Endpoints</div>
        </div>
        <div className="stat">
          <div className="stat-value">{events.length}</div>
          <div className="stat-label">Recent events</div>
        </div>
      </div>

      <section className="overview-section">
        <div className="overview-section-head">
          <h2>Recent events</h2>
          <Link to="/events" className="overview-link">
            View all
          </Link>
        </div>
        {events.length === 0 ? (
          <EmptyState
            title="No webhook events yet"
            description="Send a request to your endpoint to start capturing events."
          />
        ) : (
          <div className="overview-list">
            {events.map((ev) => (
              <Link to={`/events/${ev.id}`} key={ev.id} className="overview-row">
                <Badge tone={methodTone(ev.request_method)}>{ev.request_method}</Badge>
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

function methodTone(method: string): 'green' | 'blue' | 'yellow' | 'red' | 'neutral' {
  switch (method) {
    case 'GET':
      return 'blue';
    case 'POST':
      return 'green';
    case 'PUT':
    case 'PATCH':
      return 'yellow';
    case 'DELETE':
      return 'red';
    default:
      return 'neutral';
  }
}
