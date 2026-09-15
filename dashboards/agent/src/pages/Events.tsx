import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type StoredEvent, type ServerProject, type ServerEndpoint } from '../lib/api';
import { IconRefresh } from '../components/Icons';

export function Events() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [events, setEvents] = useState<StoredEvent[]>([]);
  const [endpointMap, setEndpointMap] = useState<Record<string, ServerEndpoint>>({});
  const [projectMap, setProjectMap] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [ev, p] = await Promise.all([
        api.get<{ events: StoredEvent[] }>('/api/events'),
        api.get<{ projects: ServerProject[] }>('/api/projects'),
      ]);
      setEvents(ev.events);
      const epMap: Record<string, ServerEndpoint> = {};
      const pMap: Record<string, string> = {};
      for (const proj of p.projects ?? []) {
        pMap[proj.id] = proj.name;
        for (const e of proj.endpoints ?? []) {
          epMap[e.id] = e;
        }
      }
      setEndpointMap(epMap);
      setProjectMap(pMap);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load events');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <h1>Events</h1>
            <p>Captured webhook events stored locally on this agent.</p>
          </div>
          <button type="button" className="btn btn-sm" onClick={load}>
            <IconRefresh size={14} /> Refresh
          </button>
        </div>
      </div>

      {error ? (
        <div className="card error-card">
          <div className="error-title">Unable to load events</div>
          <div className="error-detail">{error}</div>
          <button className="btn btn-sm" onClick={load}>
            Retry
          </button>
        </div>
      ) : (
        <div className="card">
          {loading ? (
            <div className="empty-state">Loading...</div>
          ) : events.length === 0 ? (
            <div className="empty-state">
              No events stored locally. The agent stores events for projects it
              is subscribed to.
            </div>
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th scope="col">Method</th>
                    <th scope="col">Project</th>
                    <th scope="col">Endpoint</th>
                    <th scope="col">Content type</th>
                    <th scope="col">Size</th>
                    <th scope="col">Received</th>
                  </tr>
                </thead>
                <tbody>
                  {events.map((e) => (
                    <tr key={e.id}>
                      <td>
                        <Link to={`/events/${e.id}`} className="row-link">
                          <span className="badge badge-pending">
                            {e.request_method}
                          </span>
                        </Link>
                      </td>
                      <td>
                        <Link to={`/events/${e.id}`} className="row-link">
                          {projectMap[e.project_id] ?? (
                            <span className="text-tertiary">{e.project_id.slice(0, 8)}</span>
                          )}
                        </Link>
                      </td>
                      <td>
                        <Link to={`/events/${e.id}`} className="row-link">
                          {endpointMap[e.endpoint_id]?.name ?? (
                            <span className="text-tertiary">
                              {e.endpoint_id ? e.endpoint_id.slice(0, 8) : '-'}
                            </span>
                          )}
                        </Link>
                      </td>
                      <td className="mono">
                        <Link to={`/events/${e.id}`} className="row-link">
                          {e.content_type ?? '-'}
                        </Link>
                      </td>
                      <td>
                        <Link to={`/events/${e.id}`} className="row-link">
                          {e.payload_size}
                        </Link>
                      </td>
                      <td className="mono">
                        <Link to={`/events/${e.id}`} className="row-link">
                          {e.received_at}
                        </Link>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
