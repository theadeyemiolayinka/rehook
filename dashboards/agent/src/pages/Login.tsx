import { useState } from 'react';
import { api, ApiError, type LoginRequest } from '../lib/api';
import { Logo } from '../components/Logo';

export function Login({ onLoggedIn }: { onLoggedIn: () => void }) {
  const [server, setServer] = useState('');
  const [agentId, setAgentId] = useState('');
  const [token, setToken] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const req: LoginRequest = {
        server: server.trim(),
        agent_id: agentId.trim(),
        token: token.trim(),
      };
      if (name.trim()) {
        req.name = name.trim();
      }
      await api.post('/api/auth/login', req);
      onLoggedIn();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'login failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-brand">
          <Logo size={28} withWordmark />
        </div>

        <div className="login-heading">
          <h1>Connect to a HookRelay server</h1>
          <p className="login-subtitle">
            Enter your agent credentials. The agent ID and token are shown in the
            admin dashboard when you create an agent.
          </p>
        </div>

        {error && <div className="login-error">{error}</div>}

        <form onSubmit={submit} className="login-form">
          <div className="login-field">
            <label htmlFor="login-server">Server URL</label>
            <input
              id="login-server"
              value={server}
              onChange={(e) => setServer(e.target.value)}
              placeholder="https://hooks.example.com"
              required
              autoFocus
            />
          </div>
          <div className="login-field">
            <label htmlFor="login-agent-id">Agent ID</label>
            <input
              id="login-agent-id"
              value={agentId}
              onChange={(e) => setAgentId(e.target.value)}
              placeholder="550e8400-e29b-41d4-a716-446655440000"
              required
            />
          </div>
          <div className="login-field">
            <label htmlFor="login-token">Agent token</label>
            <input
              id="login-token"
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="hr_..."
              required
            />
          </div>
          <div className="login-field">
            <label htmlFor="login-name">Agent name (optional)</label>
            <input
              id="login-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-laptop"
            />
          </div>
          <button
            type="submit"
            className="btn btn-primary login-submit"
            disabled={loading}
          >
            {loading ? 'Connecting...' : 'Connect'}
          </button>
        </form>

        <div className="login-help">
          The token is stored in your OS keychain. It is never sent to the
          browser after login.
        </div>
      </div>
    </div>
  );
}
