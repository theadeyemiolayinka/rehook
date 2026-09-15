import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import {
  api,
  ApiError,
  type EventListRow,
  type Project,
  type Endpoint,
  type Agent,
} from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Button } from '../components/Button';
import { Badge } from '../components/Badge';
import { StatusIndicator } from '../components/StatusIndicator';
import { formatBytes, formatRelative } from '../lib/format';
import './Overview.css';

export function Overview() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [events, setEvents] = useState<EventListRow[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [p, e, ev, a] = await Promise.all([
        api.get<{ projects: Project[] }>('/api/projects'),
        api.get<{ endpoints: Endpoint[] }>('/api/endpoints'),
        api.get<{ events: EventListRow[] }>('/api/events?limit=10'),
        api.get<{ agents: Agent[] }>('/api/agents'),
      ]);
      setProjects(p.projects);
      setEndpoints(e.endpoints);
      setEvents(ev.events);
      setAgents(a.agents);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load overview');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div className="overview">
        <PageHeader title="Overview" />
        <div className="overview-loading">
          <Loading />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="overview">
        <PageHeader title="Overview" />
        <EmptyState
          title="Unable to load overview"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      </div>
    );
  }

  const connectedAgents = agents.filter((a) => a.enabled && a.connected);

  return (
    <div className="overview">
      <PageHeader
        title="Overview"
        subtitle="Recent activity across your webhook projects."
      />

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
          <div className="stat-value">{connectedAgents.length}</div>
          <div className="stat-label">Connected agents</div>
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
                <Badge tone={methodTone(ev.request_method)}>
                  {ev.request_method}
                </Badge>
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

      <section className="overview-section">
        <div className="overview-section-head">
          <h2>Agents</h2>
          <Link to="/agents" className="overview-link">
            View all
          </Link>
        </div>
        {agents.length === 0 ? (
          <EmptyState
            title="No agents yet"
            description="Create an agent and connect it to receive delivery instructions."
          />
        ) : (
          <div className="overview-list">
            {agents.slice(0, 5).map((a) => (
              <div key={a.id} className="overview-row overview-row-agent">
                <span className="overview-agent-name">{a.name}</span>
                {a.enabled ? (
                  <StatusIndicator
                    state={a.connected ? 'connected' : 'disconnected'}
                    label={a.connected ? 'Connected' : 'Disconnected'}
                  />
                ) : (
                  <StatusIndicator state="disabled" label="Disabled" />
                )}
                <span className="overview-row-time">
                  {a.last_seen_at ? `seen ${formatRelative(a.last_seen_at)}` : '-'}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function methodTone(
  method: string,
): 'green' | 'blue' | 'yellow' | 'red' | 'neutral' {
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
