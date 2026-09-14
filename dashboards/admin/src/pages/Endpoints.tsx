import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, type Endpoint, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { CopyButton } from '../components/CopyButton';
import { formatRelative } from '../lib/format';
import './Endpoints.css';

export function Endpoints() {
  const [loading, setLoading] = useState(true);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [projects, setProjects] = useState<Record<string, Project>>({});

  useEffect(() => {
    async function load() {
      try {
        const [e, p] = await Promise.all([
          api.get<{ endpoints: Endpoint[] }>('/api/endpoints'),
          api.get<{ projects: Project[] }>('/api/projects'),
        ]);
        setEndpoints(e.endpoints);
        const map: Record<string, Project> = {};
        for (const proj of p.projects) map[proj.id] = proj;
        setProjects(map);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (loading) return <Loading />;

  const baseUrl = (import.meta.env.VITE_PUBLIC_BASE_URL as string | undefined) ?? '';

  return (
    <div className="endpoints">
      <PageHeader title="Endpoints" subtitle="All inbound webhook endpoints." />
      {endpoints.length === 0 ? (
        <EmptyState
          title="No endpoints yet"
          description="Create a project, then add an inbound endpoint to it."
        />
      ) : (
        <div className="endpoint-table">
          <div className="endpoint-table-head">
            <div>Name</div>
            <div>Project</div>
            <div>Webhook URL</div>
            <div>Status</div>
            <div>Created</div>
          </div>
          {endpoints.map((ep) => (
            <div key={ep.id} className="endpoint-table-row">
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
                <code>{baseUrl}/i/{ep.public_identifier}</code>
                <CopyButton value={`${baseUrl}/i/${ep.public_identifier}`} />
              </div>
              <div>
                {ep.enabled ? <Badge tone="green">Enabled</Badge> : <Badge>Disabled</Badge>}
              </div>
              <div className="et-time">{formatRelative(ep.created_at)}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
