import { useCallback, useEffect, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { api, ApiError, type EventListRow, type Endpoint, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { Button } from '../components/Button';
import { formatBytes, formatRelative } from '../lib/format';
import './Events.css';

export function Events() {
  const [params] = useSearchParams();
  const projectId = params.get('project_id') ?? undefined;
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [events, setEvents] = useState<EventListRow[]>([]);
  const [endpoints, setEndpoints] = useState<Record<string, Endpoint>>({});
  const [projects, setProjects] = useState<Record<string, Project>>({});

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const query = projectId ? `?project_id=${projectId}` : '?limit=100';
      const [e, ep, p] = await Promise.all([
        api.get<{ events: EventListRow[] }>(`/api/events${query}`),
        api.get<{ endpoints: Endpoint[] }>('/api/endpoints'),
        api.get<{ projects: Project[] }>('/api/projects'),
      ]);
      setEvents(e.events);
      const emap: Record<string, Endpoint> = {};
      for (const x of ep.endpoints) emap[x.id] = x;
      setEndpoints(emap);
      const pmap: Record<string, Project> = {};
      for (const x of p.projects) pmap[x.id] = x;
      setProjects(pmap);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load events');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div className="events">
        <PageHeader
          title="Events"
          subtitle={
            projectId && projects[projectId]
              ? `Filtered by ${projects[projectId].name}`
              : 'All captured webhook events.'
          }
        />
        <div className="events-loading">
          <Loading />
        </div>
      </div>
    );
  }

  return (
    <div className="events">
      <PageHeader
        title="Events"
        subtitle={
          projectId && projects[projectId]
            ? `Filtered by ${projects[projectId].name}`
            : 'All captured webhook events.'
        }
      />
      {error ? (
        <EmptyState
          title="Unable to load events"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      ) : events.length === 0 ? (
        <EmptyState
          title="No events yet"
          description="Send a request to an endpoint to start capturing events."
        />
      ) : (
        <div className="event-table">
          <div className="event-table-head" role="row">
            <div role="columnheader">Method</div>
            <div role="columnheader">Endpoint</div>
            <div role="columnheader">Project</div>
            <div role="columnheader">Content type</div>
            <div role="columnheader">Size</div>
            <div role="columnheader">State</div>
            <div role="columnheader">Received</div>
          </div>
          {events.map((ev) => (
            <Link to={`/events/${ev.id}`} key={ev.id} className="event-table-row" role="row">
              <div>
                <Badge tone={methodTone(ev.request_method)}>
                  {ev.request_method}
                </Badge>
              </div>
              <div className="et-endpoint">
                {endpoints[ev.endpoint_id]?.name ?? 'unknown'}
              </div>
              <div className="et-project">
                {projects[ev.project_id] ? (
                  <Link
                    to={`/projects/${ev.project_id}`}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {projects[ev.project_id].name}
                  </Link>
                ) : (
                  <span className="text-tertiary">-</span>
                )}
              </div>
              <div className="et-content-type">
                {ev.content_type ?? <span className="text-tertiary">-</span>}
              </div>
              <div className="et-size">{formatBytes(ev.payload_size)}</div>
              <div>
                <Badge tone={stateTone(ev.delivery_state)}>
                  {ev.delivery_state}
                </Badge>
              </div>
              <div className="et-time">{formatRelative(ev.received_at)}</div>
            </Link>
          ))}
        </div>
      )}
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

function stateTone(state: string): 'green' | 'yellow' | 'red' | 'neutral' {
  switch (state) {
    case 'delivered':
      return 'green';
    case 'pending':
      return 'neutral';
    case 'failed':
      return 'red';
    case 'dispatched':
      return 'yellow';
    default:
      return 'neutral';
  }
}
