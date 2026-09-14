import { useEffect, useState } from 'react';
import { api, ApiError, type StoredEvent } from '../lib/api';

export function Events() {
  const [events, setEvents] = useState<StoredEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<StoredEvent | null>(null);

  const load = async () => {
    try {
      const data = await api.get<{ events: StoredEvent[] }>('/api/events');
      setEvents(data.events);
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to load events');
    }
  };

  useEffect(() => {
    load();
  }, []);

  const replay = async (eventId: string) => {
    const targetId = prompt('Enter target ID to replay to:');
    if (!targetId) return;
    try {
      await api.post(`/api/events/${encodeURIComponent(eventId)}/replay`, { target_id: targetId });
      alert('Replay triggered. Check History for the result.');
    } catch (e) {
      alert(e instanceof ApiError ? e.message : 'replay failed');
    }
  };

  const formatBody = (body: string | null) => {
    if (!body) return '(empty)';
    try {
      const decoded = atob(body);
      try {
        return JSON.stringify(JSON.parse(decoded), null, 2);
      } catch {
        return decoded;
      }
    } catch {
      return body;
    }
  };

  return (
    <div>
      <div className="page-header">
        <h1>Events</h1>
        <p>Captured webhook events stored locally on this agent</p>
      </div>

      {error && <div className="card toast toast-error">{error}</div>}

      {selected ? (
        <div>
          <div className="card">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-3)' }}>
              <div className="card-title" style={{ margin: 0 }}>Event Detail</div>
              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                <button className="btn btn-primary btn-sm" onClick={() => replay(selected.id)}>
                  Replay
                </button>
                <button className="btn btn-sm" onClick={() => setSelected(null)}>
                  Close
                </button>
              </div>
            </div>
            <div className="status-row">
              <span className="status-label">Event ID</span>
              <span className="status-value">{selected.id}</span>
            </div>
            <div className="status-row">
              <span className="status-label">Method</span>
              <span className="status-value">{selected.request_method}</span>
            </div>
            <div className="status-row">
              <span className="status-label">Content-Type</span>
              <span className="status-value">{selected.content_type ?? 'none'}</span>
            </div>
            <div className="status-row">
              <span className="status-label">Received</span>
              <span className="status-value">{selected.received_at}</span>
            </div>
            <div className="status-row">
              <span className="status-label">Size</span>
              <span className="status-value">{selected.payload_size} bytes</span>
            </div>
          </div>

          <div className="card">
            <div className="card-title">Headers</div>
            <div className="code-block">
              {JSON.stringify(selected.headers, null, 2)}
            </div>
          </div>

          <div className="card">
            <div className="card-title">Body</div>
            <div className="code-block">{formatBody(selected.body)}</div>
          </div>
        </div>
      ) : (
        <div className="card">
          <div className="card-title">Stored Events</div>
          {events.length === 0 ? (
            <div className="empty-state">No events stored locally</div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Method</th>
                  <th>Content-Type</th>
                  <th>Size</th>
                  <th>Received</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {events.map((e) => (
                  <tr key={e.id} onClick={() => setSelected(e)} style={{ cursor: 'pointer' }}>
                    <td><span className="badge badge-pending">{e.request_method}</span></td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>
                      {e.content_type ?? '-'}
                    </td>
                    <td>{e.payload_size}</td>
                    <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>
                      {e.received_at}
                    </td>
                    <td>
                      <button className="btn btn-sm" onClick={(ev) => { ev.stopPropagation(); replay(e.id); }}>
                        Replay
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
}
