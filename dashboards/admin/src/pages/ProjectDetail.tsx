import { useCallback, useEffect, useState } from 'react';
import { Link, useParams, useNavigate } from 'react-router-dom';
import { api, ApiError, type Project, type Endpoint, type EventListRow } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Badge } from '../components/Badge';
import { CopyButton } from '../components/CopyButton';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { useToast } from '../components/Toast';
import { IconPlus, IconTrash, IconEdit } from '../components/Icons';
import { formatBytes, formatRelative } from '../lib/format';
import { usePublicBaseUrl } from '../lib/baseUrl';
import './ProjectDetail.css';

const VALIDATION_TYPES = [
  { value: 'none', label: 'None' },
  { value: 'hmac_sha256', label: 'HMAC-SHA256 signature' },
  { value: 'hmac_sha512', label: 'HMAC-SHA512 signature' },
  { value: 'header_token', label: 'Header token' },
  { value: 'query_token', label: 'Query parameter token' },
];

export function ProjectDetail() {
  const { id } = useParams<{ id: string }>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [events, setEvents] = useState<EventListRow[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [editTarget, setEditTarget] = useState<Endpoint | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Endpoint | null>(null);
  const [creating, setCreating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editProjectOpen, setEditProjectOpen] = useState(false);
  const [deleteProjectOpen, setDeleteProjectOpen] = useState(false);
  const [editProjName, setEditProjName] = useState('');
  const [editProjDesc, setEditProjDesc] = useState('');
  const [editProjEnabled, setEditProjEnabled] = useState(true);
  const [savingProject, setSavingProject] = useState(false);
  const [deletingProject, setDeletingProject] = useState(false);
  const navigate = useNavigate();
  const baseUrl = usePublicBaseUrl();
  const toast = useToast();

  // Create form state
  const [name, setName] = useState('');
  const [provider, setProvider] = useState('');
  const [valType, setValType] = useState('none');
  const [valSecret, setValSecret] = useState('');
  const [valHeader, setValHeader] = useState('');
  const [valQuery, setValQuery] = useState('');

  // Edit form state
  const [editName, setEditName] = useState('');
  const [editEnabled, setEditEnabled] = useState(true);
  const [editProvider, setEditProvider] = useState('');
  const [editValType, setEditValType] = useState('none');
  const [editValSecret, setEditValSecret] = useState('');
  const [editValHeader, setEditValHeader] = useState('');
  const [editValQuery, setEditValQuery] = useState('');

  const load = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    setError(null);
    try {
      const [p, e, ev] = await Promise.all([
        api.get<Project>(`/api/projects/${id}`),
        api.get<{ endpoints: Endpoint[] }>(`/api/projects/${id}/endpoints`),
        api.get<{ events: EventListRow[] }>(
          `/api/events?project_id=${id}&limit=10`,
        ),
      ]);
      setProject(p);
      setEndpoints(e.endpoints);
      setEvents(ev.events);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load project');
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  function openEdit(ep: Endpoint) {
    setEditTarget(ep);
    setEditName(ep.name);
    setEditEnabled(ep.enabled);
    setEditProvider(ep.provider ?? '');
    setEditValType(ep.validation_type ?? 'none');
    setEditValSecret(ep.validation_secret ?? '');
    setEditValHeader(ep.validation_header ?? '');
    setEditValQuery(ep.validation_query ?? '');
  }

  async function handleCreate() {
    if (!id) return;
    setCreating(true);
    try {
      await api.post(`/api/projects/${id}/endpoints`, {
        name: name.trim(),
        provider: provider.trim() || null,
        validation_type: valType,
        validation_secret: valSecret.trim() || null,
        validation_header: valHeader.trim() || null,
        validation_query: valQuery.trim() || null,
      });
      setName('');
      setProvider('');
      setValType('none');
      setValSecret('');
      setValHeader('');
      setValQuery('');
      setCreateOpen(false);
      toast.show('Endpoint created', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Create failed',
        'error',
      );
    } finally {
      setCreating(false);
    }
  }

  async function handleEdit() {
    if (!editTarget) return;
    setSaving(true);
    try {
      await api.patch(`/api/endpoints/${editTarget.id}`, {
        name: editName.trim(),
        enabled: editEnabled,
        provider: editProvider.trim() || null,
        validation_type: editValType,
        validation_secret: editValSecret.trim() || null,
        validation_header: editValHeader.trim() || null,
        validation_query: editValQuery.trim() || null,
      });
      toast.show('Endpoint updated', 'success');
      setEditTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Update failed',
        'error',
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.delete(`/api/endpoints/${deleteTarget.id}`);
      toast.show('Endpoint deleted', 'success');
      setDeleteTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Delete failed',
        'error',
      );
    } finally {
      setDeleting(false);
    }
  }

  function openProjectEdit() {
    if (!project) return;
    setEditProjName(project.name);
    setEditProjDesc(project.description ?? '');
    setEditProjEnabled(project.enabled);
    setEditProjectOpen(true);
  }

  async function handleProjectEdit() {
    if (!project || !editProjName.trim()) return;
    setSavingProject(true);
    try {
      await api.patch(`/api/projects/${project.id}`, {
        name: editProjName.trim(),
        description: editProjDesc.trim(),
        enabled: editProjEnabled,
      });
      toast.show('Project updated', 'success');
      setEditProjectOpen(false);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Update failed',
        'error',
      );
    } finally {
      setSavingProject(false);
    }
  }

  async function handleProjectDelete() {
    if (!project) return;
    setDeletingProject(true);
    try {
      await api.delete(`/api/projects/${project.id}`);
      toast.show('Project deleted', 'success');
      navigate('/projects');
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Delete failed',
        'error',
      );
      setDeletingProject(false);
      setDeleteProjectOpen(false);
    }
  }

  if (loading) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <div className="project-detail-loading">
          <Loading />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <EmptyState
          title="Unable to load project"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      </div>
    );
  }

  if (!project) {
    return (
      <div className="project-detail">
        <PageHeader title="Project" />
        <EmptyState title="Project not found" />
      </div>
    );
  }

  const showSecretField = (type: string) =>
    type === 'hmac_sha256' || type === 'hmac_sha512' || type === 'header_token' || type === 'query_token';
  const showHeaderField = (type: string) =>
    type === 'hmac_sha256' || type === 'hmac_sha512' || type === 'header_token';
  const showQueryField = (type: string) => type === 'query_token';

  const validationLabel = (type: string) => {
    const vt = VALIDATION_TYPES.find((v) => v.value === type);
    return vt ? vt.label : type;
  };

  return (
    <div className="project-detail">
      <PageHeader
        title={project.name}
        subtitle={project.description || 'No description'}
        actions={
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={openProjectEdit}
              aria-label="Edit project"
            >
              <IconEdit size={14} />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setDeleteProjectOpen(true)}
              aria-label="Delete project"
            >
              <IconTrash size={14} />
            </Button>
            <Button variant="primary" size="sm" onClick={() => setCreateOpen(true)}>
              <IconPlus size={14} /> New endpoint
            </Button>
          </>
        }
      />

      <section className="detail-section">
        <h2 className="detail-section-title">Endpoints</h2>
        {endpoints.length === 0 ? (
          <EmptyState
            title="No endpoints yet"
            description="Create an inbound endpoint to receive webhooks for this project."
            action={
              <Button variant="primary" onClick={() => setCreateOpen(true)}>
                <IconPlus size={14} /> New endpoint
              </Button>
            }
          />
        ) : (
          <div className="endpoint-list">
            {endpoints.map((ep) => (
              <div key={ep.id} className="endpoint-row">
                <div className="endpoint-row-main">
                  <div className="endpoint-row-head">
                    <span className="endpoint-row-name">{ep.name}</span>
                    {ep.enabled ? (
                      <Badge tone="green">Enabled</Badge>
                    ) : (
                      <Badge>Disabled</Badge>
                    )}
                    {ep.provider ? (
                      <Badge tone="blue">{ep.provider}</Badge>
                    ) : null}
                    {ep.validation_type !== 'none' ? (
                      <Badge tone="yellow">{validationLabel(ep.validation_type)}</Badge>
                    ) : null}
                  </div>
                  <div className="endpoint-row-url">
                    <code>
                      {baseUrl}/i/{ep.public_identifier}
                    </code>
                    <CopyButton
                      value={`${baseUrl}/i/${ep.public_identifier}`}
                      compact
                    />
                  </div>
                </div>
                <div className="endpoint-row-actions">
                  <button
                    type="button"
                    className="endpoint-row-edit"
                    onClick={() => openEdit(ep)}
                    aria-label={`Edit endpoint ${ep.name}`}
                  >
                    <IconEdit size={14} />
                  </button>
                  <button
                    type="button"
                    className="endpoint-row-delete"
                    onClick={() => setDeleteTarget(ep)}
                    aria-label={`Delete endpoint ${ep.name}`}
                  >
                    <IconTrash size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="detail-section">
        <div className="detail-section-head">
          <h2 className="detail-section-title">Recent events</h2>
          {events.length > 0 ? (
            <Link
              to={`/events?project_id=${project.id}`}
              className="overview-link"
            >
              View all
            </Link>
          ) : null}
        </div>
        {events.length === 0 ? (
          <EmptyState
            title="No events yet"
            description="Send a request to an endpoint above to capture a webhook."
          />
        ) : (
          <div className="overview-list">
            {events.map((ev) => (
              <Link
                to={`/events/${ev.id}`}
                key={ev.id}
                className="overview-row"
              >
                <Badge tone="green">{ev.request_method}</Badge>
                <span className="overview-row-method">
                  {ev.content_type ?? 'no content type'}
                </span>
                <span className="overview-row-size">
                  {formatBytes(ev.payload_size)}
                </span>
                <span className="overview-row-time">
                  {formatRelative(ev.received_at)}
                </span>
              </Link>
            ))}
          </div>
        )}
      </section>

      {/* Create endpoint dialog */}
      <Dialog
        open={createOpen}
        title="New endpoint"
        description="An inbound endpoint receives webhooks at a unique URL. Optionally configure validation to verify that requests come from the expected provider."
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
              Create endpoint
            </Button>
          </>
        }
      >
        <Input
          label="Name"
          name="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="paystack"
          autoFocus
          required
        />
        <Input
          label="Provider (optional)"
          name="provider"
          value={provider}
          onChange={(e) => setProvider(e.target.value)}
          placeholder="paystack, github, stripe"
          hint="Informational label for the webhook provider."
        />
        <div className="field">
          <label className="field-label" htmlFor="create-validation-type">
            Signature validation
          </label>
          <select
            id="create-validation-type"
            className="input"
            value={valType}
            onChange={(e) => setValType(e.target.value)}
          >
            {VALIDATION_TYPES.map((vt) => (
              <option key={vt.value} value={vt.value}>
                {vt.label}
              </option>
            ))}
          </select>
          <span className="field-hint">
            Optionally verify that inbound webhooks are authentic. The provider must support the selected method.
          </span>
        </div>
        {showSecretField(valType) ? (
          <Input
            label={valType === 'query_token' ? 'Expected token value' : 'Secret'}
            name="validation_secret"
            value={valSecret}
            onChange={(e) => setValSecret(e.target.value)}
            placeholder={valType === 'query_token' ? 'token value' : 'whsec_... or shared secret'}
            hint={
              valType === 'hmac_sha256' || valType === 'hmac_sha512'
                ? 'The shared secret used to verify the HMAC signature.'
                : 'The expected token value.'
            }
          />
        ) : null}
        {showHeaderField(valType) ? (
          <Input
            label="Signature header name"
            name="validation_header"
            value={valHeader}
            onChange={(e) => setValHeader(e.target.value)}
            placeholder={
              valType === 'hmac_sha256'
                ? 'x-hub-signature-256'
                : valType === 'hmac_sha512'
                  ? 'x-paystack-signature'
                  : 'x-api-key'
            }
            hint="The header that carries the signature or token."
          />
        ) : null}
        {showQueryField(valType) ? (
          <Input
            label="Query parameter name"
            name="validation_query"
            value={valQuery}
            onChange={(e) => setValQuery(e.target.value)}
            placeholder="token"
            hint="The query parameter that carries the token (e.g. ?token=abc123)."
          />
        ) : null}
      </Dialog>

      {/* Edit endpoint dialog */}
      <Dialog
        open={!!editTarget}
        title={`Edit endpoint`}
        description={editTarget ? `Update settings for ${editTarget.name}.` : ''}
        onClose={() => setEditTarget(null)}
        footer={
          <>
            <Button
              variant="ghost"
              onClick={() => setEditTarget(null)}
              disabled={saving}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleEdit}
              loading={saving}
              disabled={!editName.trim()}
            >
              Save changes
            </Button>
          </>
        }
      >
        {editTarget ? (
          <>
            <Input
              label="Name"
              name="edit_name"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              autoFocus
              required
            />
            <div className="field">
              <label className="field-label" htmlFor="edit-enabled">
                Status
              </label>
              <select
                id="edit-enabled"
                className="input"
                value={editEnabled ? 'enabled' : 'disabled'}
                onChange={(e) => setEditEnabled(e.target.value === 'enabled')}
              >
                <option value="enabled">Enabled</option>
                <option value="disabled">Disabled</option>
              </select>
            </div>
            <Input
              label="Provider (optional)"
              name="edit_provider"
              value={editProvider}
              onChange={(e) => setEditProvider(e.target.value)}
              placeholder="paystack, github, stripe"
            />
            <div className="field">
              <label className="field-label" htmlFor="edit-validation-type">
                Signature validation
              </label>
              <select
                id="edit-validation-type"
                className="input"
                value={editValType}
                onChange={(e) => setEditValType(e.target.value)}
              >
                {VALIDATION_TYPES.map((vt) => (
                  <option key={vt.value} value={vt.value}>
                    {vt.label}
                  </option>
                ))}
              </select>
            </div>
            {showSecretField(editValType) ? (
              <Input
                label={editValType === 'query_token' ? 'Expected token value' : 'Secret'}
                name="edit_validation_secret"
                value={editValSecret}
                onChange={(e) => setEditValSecret(e.target.value)}
                placeholder={editValType === 'query_token' ? 'token value' : 'whsec_... or shared secret'}
              />
            ) : null}
            {showHeaderField(editValType) ? (
              <Input
                label="Signature header name"
                name="edit_validation_header"
                value={editValHeader}
                onChange={(e) => setEditValHeader(e.target.value)}
                placeholder={
                  editValType === 'hmac_sha256'
                    ? 'x-hub-signature-256'
                    : editValType === 'hmac_sha512'
                      ? 'x-paystack-signature'
                      : 'x-api-key'
                }
              />
            ) : null}
            {showQueryField(editValType) ? (
              <Input
                label="Query parameter name"
                name="edit_validation_query"
                value={editValQuery}
                onChange={(e) => setEditValQuery(e.target.value)}
                placeholder="token"
              />
            ) : null}
          </>
        ) : null}
      </Dialog>

      {/* Edit project dialog */}
      <Dialog
        open={editProjectOpen}
        title="Edit project"
        description="Update the project name, description, or status. Disabling a project rejects all inbound webhooks on its endpoints."
        onClose={() => setEditProjectOpen(false)}
        footer={
          <>
            <Button
              variant="ghost"
              onClick={() => setEditProjectOpen(false)}
              disabled={savingProject}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleProjectEdit}
              loading={savingProject}
              disabled={!editProjName.trim()}
            >
              Save changes
            </Button>
          </>
        }
      >
        <Input
          label="Name"
          name="edit_proj_name"
          value={editProjName}
          onChange={(e) => setEditProjName(e.target.value)}
          autoFocus
          required
        />
        <Input
          label="Description"
          name="edit_proj_desc"
          value={editProjDesc}
          onChange={(e) => setEditProjDesc(e.target.value)}
          placeholder="Optional"
        />
        <div className="field">
          <label className="field-label" htmlFor="edit-proj-enabled">
            Status
          </label>
          <select
            id="edit-proj-enabled"
            className="input"
            value={editProjEnabled ? 'enabled' : 'disabled'}
            onChange={(e) => setEditProjEnabled(e.target.value === 'enabled')}
          >
            <option value="enabled">Enabled</option>
            <option value="disabled">Disabled</option>
          </select>
        </div>
      </Dialog>

      <ConfirmDialog
        open={deleteProjectOpen}
        title="Delete project"
        description={`Delete ${project.name}? All endpoints and captured events in this project will also be deleted. This cannot be undone.`}
        confirmLabel="Delete project"
        destructive
        loading={deletingProject}
        onConfirm={handleProjectDelete}
        onCancel={() => setDeleteProjectOpen(false)}
      />

      <ConfirmDialog
        open={!!deleteTarget}
        title="Delete endpoint"
        description={
          deleteTarget
            ? `Delete ${deleteTarget.name}? All captured events for this endpoint will also be deleted. This cannot be undone.`
            : ''
        }
        confirmLabel="Delete endpoint"
        destructive
        loading={deleting}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}
