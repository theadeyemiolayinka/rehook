import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ApiError, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { Dialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconPlus } from '../components/Icons';
import { formatRelative } from '../lib/format';
import './Projects.css';

export function Projects() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [creating, setCreating] = useState(false);
  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await api.get<{ projects: Project[] }>('/api/projects');
      setProjects(res.projects);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load projects');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function handleCreate() {
    setCreating(true);
    try {
      await api.post('/api/projects', { name: name.trim(), description: description.trim() });
      setName('');
      setDescription('');
      setCreateOpen(false);
      toast.show('Project created', 'success');
      await load();
    } catch (e) {
      toast.show(e instanceof ApiError ? e.message : 'Create failed', 'error');
    } finally {
      setCreating(false);
    }
  }

  if (loading) {
    return (
      <div className="projects">
        <PageHeader title="Projects" subtitle="Logical webhook workspaces." />
        <div className="projects-loading">
          <Loading />
        </div>
      </div>
    );
  }

  return (
    <div className="projects">
      <PageHeader
        title="Projects"
        subtitle="Logical webhook workspaces."
        actions={
          <Button variant="primary" size="sm" onClick={() => setCreateOpen(true)}>
            <IconPlus size={14} /> New project
          </Button>
        }
      />

      {error ? (
        <EmptyState
          title="Unable to load projects"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      ) : projects.length === 0 ? (
        <EmptyState
          title="No projects yet"
          description="Create a project to group related webhook endpoints."
          action={
            <Button variant="primary" onClick={() => setCreateOpen(true)}>
              <IconPlus size={14} /> New project
            </Button>
          }
        />
      ) : (
        <div className="project-list">
          {projects.map((p) => (
            <Link to={`/projects/${p.id}`} key={p.id} className="project-card">
              <div className="project-card-head">
                <span className="project-card-name">{p.name}</span>
                {p.enabled ? (
                  <Badge tone="green">Enabled</Badge>
                ) : (
                  <Badge>Disabled</Badge>
                )}
              </div>
              {p.description ? (
                <div className="project-card-desc">{p.description}</div>
              ) : null}
              <div className="project-card-meta">
                <span className="project-card-slug">{p.slug}</span>
                <span>created {formatRelative(p.created_at)}</span>
              </div>
            </Link>
          ))}
        </div>
      )}

      <Dialog
        open={createOpen}
        title="New project"
        description="A project groups related webhook endpoints."
        onClose={() => setCreateOpen(false)}
        footer={
          <>
            <Button
              variant="ghost"
              onClick={() => setCreateOpen(false)}
              disabled={creating}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleCreate}
              loading={creating}
              disabled={!name.trim()}
            >
              Create project
            </Button>
          </>
        }
      >
        <Input
          label="Name"
          name="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="payments"
          autoFocus
          required
        />
        <Input
          label="Description"
          name="description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Optional"
        />
      </Dialog>
    </div>
  );
}
