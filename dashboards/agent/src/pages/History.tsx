import { useEffect, useState } from 'react';
import { api, ApiError, type DeliveryRecord } from '../lib/api';

export function History() {
  const [records, setRecords] = useState<DeliveryRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      const data = await api.get<{ deliveries: DeliveryRecord[] }>('/api/deliveries');
      setRecords(data.deliveries);
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'failed to load history');
    }
  };

  useEffect(() => {
    load();
  }, []);

  const statusBadge = (status: string) => {
    if (status === 'delivered') return <span className="badge badge-success">{status}</span>;
    if (status === 'failed') return <span className="badge badge-error">{status}</span>;
    return <span className="badge badge-pending">{status}</span>;
  };

  return (
    <div>
      <div className="page-header">
        <h1>Delivery History</h1>
        <p>Local delivery attempts recorded by this agent</p>
      </div>

      {error && <div className="card toast toast-error">{error}</div>}

      <div className="card">
        <div className="card-title">Recent Deliveries</div>
        {records.length === 0 ? (
          <div className="empty-state">No delivery attempts recorded</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Status</th>
                <th>HTTP</th>
                <th>Duration</th>
                <th>Target</th>
                <th>Started</th>
                <th>Error</th>
              </tr>
            </thead>
            <tbody>
              {records.map((r) => (
                <tr key={r.id}>
                  <td>{statusBadge(r.status)}</td>
                  <td>{r.http_status ?? '-'}</td>
                  <td>{r.duration_ms != null ? `${r.duration_ms}ms` : '-'}</td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>
                    {r.target_id}
                  </td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)' }}>
                    {r.started_at}
                  </td>
                  <td style={{ fontSize: 'var(--text-xs)', color: 'var(--text-tertiary)' }}>
                    {r.error_message ?? '-'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
