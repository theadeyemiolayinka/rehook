import { useCallback, useEffect, useState } from 'react';
import { api, ApiError, type Agent } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { Dialog, ConfirmDialog } from '../components/Dialog';
import { StatusIndicator } from '../components/StatusIndicator';
import { CopyButton } from '../components/CopyButton';
import { useToast } from '../components/Toast';
import { IconPlus, IconRefresh } from '../components/Icons';
import { formatRelative } from '../lib/format';
import './Agents.css';

export function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<(Agent & { token: string }) | null>(null);
  const [revokeTarget, setRevokeTarget] = useState<Agent | null>(null);
  const [revoking, setRevoking] = useState(false);
  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await api.get<{ agents: Agent[] }>('/api/agents');
      setAgents(res.agents);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load agents');
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
      const res = await api.post<{ agent: Agent; token: string }>(
        '/api/agents',
        { name: newName.trim() },
      );
      setCreated({ ...res.agent, token: res.token });
      setNewName('');
      setCreateOpen(false);
      toast.show('Agent created', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to create agent',
        'error',
      );
    } finally {
      setCreating(false);
    }
  }

  async function handleRevoke() {
    if (!revokeTarget) return;
    setRevoking(true);
    try {
      await api.patch(`/api/agents/${revokeTarget.id}`, { enabled: false });
      toast.show('Agent disabled', 'success');
      setRevokeTarget(null);
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to disable agent',
        'error',
      );
    } finally {
      setRevoking(false);
    }
  }

  async function handleEnable(agent: Agent) {
    try {
      await api.patch(`/api/agents/${agent.id}`, { enabled: true });
      toast.show('Agent enabled', 'success');
      await load();
    } catch (e) {
      toast.show(
        e instanceof ApiError ? e.message : 'Failed to enable agent',
        'error',
      );
    }
  }

  return (
    <div className="agents">
      <PageHeader
        title="Agents"
        subtitle="Local agents that connect to this server to receive delivery instructions."
        actions={
          <>
            <Button variant="ghost" size="sm" onClick={load}>
              <IconRefresh size={14} /> Refresh
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => setCreateOpen(true)}
            >
              <IconPlus size={14} /> New agent
            </Button>
          </>
        }
      />

      {loading ? (
        <div className="agents-loading">
          <Loading />
        </div>
      ) : error ? (
        <EmptyState
          title="Unable to load agents"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      ) : agents.length === 0 ? (
        <EmptyState
          title="No agents yet"
          description="Create an agent, then run the HookRelay agent on your machine and sign in with the agent ID and token."
          action={
            <Button variant="primary" onClick={() => setCreateOpen(true)}>
              <IconPlus size={14} /> New agent
            </Button>
          }
        />
      ) : (
        <div className="agents-table-wrap">
          <table className="agents-table">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Status</th>
                <th scope="col">Last seen</th>
                <th scope="col">Created</th>
                <th scope="col">Agent ID</th>
                <th scope="col"></th>
              </tr>
            </thead>
            <tbody>
              {agents.map((a) => (
                <tr key={a.id}>
                  <td className="agents-name">{a.name}</td>
                  <td>
                    {a.enabled ? (
                      <StatusIndicator
                        state={a.connected ? 'connected' : 'disconnected'}
                        label={a.connected ? 'Connected' : 'Disconnected'}
                      />
                    ) : (
                      <StatusIndicator state="disabled" label="Disabled" />
                    )}
                  </td>
                  <td className="agents-muted">
                    {a.last_seen_at ? formatRelative(a.last_seen_at) : '-'}
                  </td>
                  <td className="agents-muted">
                    {formatRelative(a.created_at)}
                  </td>
                  <td className="agents-id">
                    <code>{a.id.slice(0, 8)}</code>
                    <CopyButton value={a.id} compact />
                  </td>
                  <td className="agents-actions">
                    {!a.enabled ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => handleEnable(a)}
                      >
                        Enable
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setRevokeTarget(a)}
                      >
                        Disable
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <Dialog
        open={createOpen}
        title="New agent"
        description="Create an agent credential. The token is shown only once."
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
              disabled={!newName.trim()}
            >
              Create agent
            </Button>
          </>
        }
      >
        <Input
          label="Agent name"
          name="name"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="my-laptop"
          autoFocus
        />
      </Dialog>

      <Dialog
        open={!!created}
        title="Agent created"
        description="Save these credentials now. The token will not be shown again."
        onClose={() => setCreated(null)}
        footer={
          <Button variant="primary" onClick={() => setCreated(null)}>
            Done
          </Button>
        }
      >
        {created ? (
          <div className="agents-created">
            <div className="agents-created-row">
              <span className="agents-created-label">Agent ID</span>
              <code className="agents-created-value">{created.id}</code>
              <CopyButton value={created.id} compact />
            </div>
            <div className="agents-created-row">
              <span className="agents-created-label">Token</span>
              <code className="agents-created-value agents-token">
                {created.token}
              </code>
              <CopyButton value={created.token} compact />
            </div>
            <p className="agents-created-help">
              Use these with the agent CLI or web UI login.
            </p>
          </div>
        ) : null}
      </Dialog>

      <ConfirmDialog
        open={!!revokeTarget}
        title="Disable agent"
        description={
          revokeTarget
            ? `Disable ${revokeTarget.name}? The agent will be disconnected and cannot reconnect until re-enabled.`
            : ''
        }
        confirmLabel="Disable agent"
        destructive
        loading={revoking}
        onConfirm={handleRevoke}
        onCancel={() => setRevokeTarget(null)}
      />
    </div>
  );
}
