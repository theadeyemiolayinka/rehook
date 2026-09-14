import { useEffect, useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import { api, type Project } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { useToast } from '../components/Toast';
import { formatRelative } from '../lib/format';
import './Projects.css';

export function Projects() {
  const [loading, setLoading] = useState(true);
  const [projects, setProjects] = useState<Project[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [creating, setCreating] = useState(false);
  const toast = useToast();

  async function load() {
    try {
      const res = await api.get<{ projects: Project[] }>('/api/projects');
      setProjects(res.projects);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleCreate(e: FormEvent) {
    e.preventDefault();
    setCreating(true);
    try {
      await api.post('/api/projects', { name, description });
      setName('');
      setDescription('');
      setShowCreate(false);
      toast.show('Project created', 'success');
      await load();
    } catch (err) {
      toast.show(err instanceof Error ? err.message : 'Create failed', 'error');
    } finally {
      setCreating(false);
    }
  }

  if (loading) return <Loading />;

  return (
    <div className="projects">
      <PageHeader
        title="Projects"
        subtitle="Logical webhook workspaces."
        actions={
          <Button variant="primary" onClick={() => setShowCreate((s) => !s)}>
            New project
          </Button>
        }
      />

      {showCreate ? (
        <form className="card create-form" onSubmit={handleCreate}>
          <Input
            label="Name"
            name="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Armor of Light"
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
          <div className="create-form-actions">
            <Button type="submit" variant="primary" loading={creating}>
              Create
            </Button>
            <Button type="button" variant="ghost" onClick={() => setShowCreate(false)}>
              Cancel
            </Button>
          </div>
        </form>
      ) : null}

      {projects.length === 0 && !showCreate ? (
        <EmptyState
          title="No projects yet"
          description="Create a project to group related webhook endpoints."
          action={
            <Button variant="primary" onClick={() => setShowCreate(true)}>
              New project
            </Button>
          }
        />
      ) : (
        <div className="project-list">
          {projects.map((p) => (
            <Link to={`/projects/${p.id}`} key={p.id} className="project-card">
              <div className="project-card-head">
                <span className="project-card-name">{p.name}</span>
                {p.enabled ? <Badge tone="green">Enabled</Badge> : <Badge>Disabled</Badge>}
              </div>
              {p.description ? (
                <div className="project-card-desc">{p.description}</div>
              ) : null}
              <div className="project-card-meta">
                <span>{p.slug}</span>
                <span>created {formatRelative(p.created_at)}</span>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
