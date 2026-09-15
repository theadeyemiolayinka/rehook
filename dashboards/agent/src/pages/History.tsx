import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type DeliveryRecord } from '../lib/api';
import { IconRefresh } from '../components/Icons';

export function History() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [records, setRecords] = useState<DeliveryRecord[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.get<{ deliveries: DeliveryRecord[] }>(
        '/api/deliveries',
      );
      setRecords(data.deliveries);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load history');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const statusBadge = (status: string) => {
    if (status === 'delivered')
      return <span className="badge badge-success">{status}</span>;
    if (status === 'failed')
      return <span className="badge badge-error">{status}</span>;
    return <span className="badge badge-pending">{status}</span>;
  };

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <h1>Delivery history</h1>
            <p>Local delivery attempts recorded by this agent.</p>
          </div>
          <button type="button" className="btn btn-sm" onClick={load}>
            <IconRefresh size={14} /> Refresh
          </button>
        </div>
      </div>

      {error ? (
        <div className="card error-card">
          <div className="error-title">Unable to load history</div>
          <div className="error-detail">{error}</div>
          <button className="btn btn-sm" onClick={load}>
            Retry
          </button>
        </div>
      ) : (
        <div className="card">
          <div className="card-title">Recent deliveries</div>
          {loading ? (
            <div className="empty-state">Loading...</div>
          ) : records.length === 0 ? (
            <div className="empty-state">
              No delivery attempts recorded. Replay an event on the Events page
              to deliver it locally.
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
                    <th scope="col">Event</th>
                    <th scope="col">Started</th>
                    <th scope="col">Error</th>
                  </tr>
                </thead>
                <tbody>
                  {records.map((r) => (
                    <tr key={r.id}>
                      <td>{statusBadge(r.status)}</td>
                      <td className="mono">{r.http_status ?? '-'}</td>
                      <td className="mono">
                        {r.duration_ms != null ? `${r.duration_ms}ms` : '-'}
                      </td>
                      <td className="mono">{r.target_id}</td>
                      <td className="mono">
                        <Link to={`/events/${r.event_id}`} className="row-link">
                          {r.event_id.slice(0, 8)}
                        </Link>
                      </td>
                      <td className="mono">{r.started_at}</td>
                      <td className="error-cell">
                        {r.error_message ?? '-'}
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
