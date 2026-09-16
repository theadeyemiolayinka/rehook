import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type DeliveryRecord } from '../lib/api';
import { CodeViewer } from '../components/CodeViewer';
import { IconArrowLeft } from '../components/Icons';
import '../components/CodeViewer.css';

function decodeBase64(b64: string): string {
  try {
    return atob(b64);
  } catch {
    return '';
  }
}

function statusBadge(status: string) {
  if (status === 'delivered')
    return <span className="badge badge-success">{status}</span>;
  if (status === 'failed')
    return <span className="badge badge-error">{status}</span>;
  return <span className="badge badge-pending">{status}</span>;
}

export function DeliveryDetail({ id }: { id: string }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [record, setRecord] = useState<DeliveryRecord | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rec = await api.get<DeliveryRecord>(`/api/deliveries/${id}`);
      setRecord(rec);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load delivery');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div>
        <div className="page-header">
          <div className="page-header-row">
            <div>
              <h1>Delivery</h1>
              <p>Loading delivery details.</p>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="empty-state">Loading...</div>
        </div>
      </div>
    );
  }

  if (error || !record) {
    return (
      <div>
        <div className="page-header">
          <div className="page-header-row">
            <div>
              <div className="page-header-back">
                <Link to="/history" className="page-header-back-link">
                  <IconArrowLeft size={14} /> History
                </Link>
              </div>
              <h1>Delivery</h1>
              <p>Could not load this delivery.</p>
            </div>
          </div>
        </div>
        <div className="card error-card">
          <div className="error-title">Unable to load delivery</div>
          <div className="error-detail">{error ?? 'Delivery not found.'}</div>
          <Link to="/history" className="btn btn-sm">
            Back to history
          </Link>
        </div>
      </div>
    );
  }

  const responseHeaders = record.response_headers ?? {};
  const responseBody = record.response_body
    ? decodeBase64(record.response_body)
    : '';
  const hasResponse =
    record.http_status != null ||
    Object.keys(responseHeaders).length > 0 ||
    responseBody.length > 0;
  const responseContentType =
    responseHeaders['content-type'] ?? responseHeaders['Content-Type'] ?? null;

  return (
    <div>
      <div className="page-header">
        <div className="page-header-row">
          <div>
            <div className="page-header-back">
              <Link to="/history" className="page-header-back-link">
                <IconArrowLeft size={14} /> History
              </Link>
            </div>
            <h1>Delivery detail</h1>
            <p>Local delivery attempt recorded by this agent.</p>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Delivery</div>
        <div className="status-row">
          <span className="status-label">Delivery ID</span>
          <span className="status-value mono">{record.id}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Status</span>
          <span className="status-value">{statusBadge(record.status)}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Event</span>
          <span className="status-value mono">
            <Link to={`/events/${record.event_id}`} className="row-link">
              {record.event_id}
            </Link>
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Target</span>
          <span className="status-value mono">{record.target_id || '-'}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Attempt</span>
          <span className="status-value mono">
            {record.attempt_number || 1}
          </span>
        </div>
        <div className="status-row">
          <span className="status-label">Started</span>
          <span className="status-value mono">{record.started_at}</span>
        </div>
        <div className="status-row">
          <span className="status-label">Completed</span>
          <span className="status-value mono">
            {record.completed_at ?? '-'}
          </span>
        </div>
        {record.error_message ? (
          <div className="status-row">
            <span className="status-label">Error</span>
            <span className="status-value error-cell">
              {record.error_category
                ? `${record.error_category}: ${record.error_message}`
                : record.error_message}
            </span>
          </div>
        ) : null}
      </div>

      <div className="card">
        <div className="card-title">Response</div>
        {!hasResponse ? (
          <div className="empty-state">
            No response was received. The request failed before a response
            arrived, or the target was unreachable.
          </div>
        ) : (
          <>
            <div className="status-row">
              <span className="status-label">HTTP status</span>
              <span className="status-value mono">
                {record.http_status ?? '-'}
              </span>
            </div>
            <div className="status-row">
              <span className="status-label">Duration</span>
              <span className="status-value mono">
                {record.duration_ms != null ? `${record.duration_ms}ms` : '-'}
              </span>
            </div>
            {Object.keys(responseHeaders).length > 0 ? (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th scope="col">Header</th>
                      <th scope="col">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {Object.entries(responseHeaders).map(([k, v]) => (
                      <tr key={k}>
                        <td className="mono text-secondary">{k}</td>
                        <td className="mono">{v}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : null}
          </>
        )}
      </div>

      {responseBody ? (
        <div className="card">
          <div className="card-title">Response body</div>
          <CodeViewer body={responseBody} contentType={responseContentType} />
        </div>
      ) : null}
    </div>
  );
}
