import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type Endpoint, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { Button } from '../components/Button';
import { CopyButton } from '../components/CopyButton';
import { formatRelative } from '../lib/format';
import './Endpoints.css';

export function Endpoints() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [projects, setProjects] = useState<Record<string, Project>>({});

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [e, p] = await Promise.all([
        api.get<{ endpoints: Endpoint[] }>('/api/endpoints'),
        api.get<{ projects: Project[] }>('/api/projects'),
      ]);
      setEndpoints(e.endpoints);
      const map: Record<string, Project> = {};
      for (const proj of p.projects) map[proj.id] = proj;
      setProjects(map);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load endpoints');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div className="endpoints">
        <PageHeader title="Endpoints" subtitle="All inbound webhook endpoints." />
        <div className="endpoints-loading">
          <Loading />
        </div>
      </div>
    );
  }

  const baseUrl = (import.meta.env.VITE_PUBLIC_BASE_URL as string | undefined) ?? '';

  return (
    <div className="endpoints">
      <PageHeader title="Endpoints" subtitle="All inbound webhook endpoints." />
      {error ? (
        <EmptyState
          title="Unable to load endpoints"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      ) : endpoints.length === 0 ? (
        <EmptyState
          title="No endpoints yet"
          description="Create a project, then add an inbound endpoint to it."
        />
      ) : (
        <div className="endpoint-table">
          <div className="endpoint-table-head" role="row">
            <div role="columnheader">Name</div>
            <div role="columnheader">Project</div>
            <div role="columnheader">Webhook URL</div>
            <div role="columnheader">Status</div>
            <div role="columnheader">Created</div>
          </div>
          {endpoints.map((ep) => (
            <div key={ep.id} className="endpoint-table-row" role="row">
              <div className="et-name">{ep.name}</div>
              <div className="et-project">
                {projects[ep.project_id] ? (
                  <Link to={`/projects/${ep.project_id}`}>
                    {projects[ep.project_id].name}
                  </Link>
                ) : (
                  <span className="text-tertiary">-</span>
                )}
              </div>
              <div className="et-url">
                <code>
                  {baseUrl}/i/{ep.public_identifier}
                </code>
                <CopyButton value={`${baseUrl}/i/${ep.public_identifier}`} compact />
              </div>
              <div>
                {ep.enabled ? (
                  <Badge tone="green">Enabled</Badge>
                ) : (
                  <Badge>Disabled</Badge>
                )}
              </div>
              <div className="et-time">{formatRelative(ep.created_at)}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
