import { useState } from 'react';
import { api, ApiError, type LoginRequest } from '../lib/api';

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
          <svg width="32" height="32" viewBox="0 0 32 32" fill="none" aria-hidden="true">
            <path d="M8 10h6a4 4 0 0 1 4 4v0a4 4 0 0 1-4 4h-2" stroke="#e6e8ea" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            <circle cx="22" cy="14" r="3" stroke="#4ade80" strokeWidth="2" fill="none"/>
            <path d="M19 14h-1" stroke="#4ade80" strokeWidth="2" strokeLinecap="round"/>
          </svg>
          <span>HookRelay Agent</span>
        </div>

        <h1>Connect to a HookRelay Server</h1>
        <p className="login-subtitle">
          Enter your agent credentials. You can find the agent ID and token in the
          admin dashboard when you create an agent.
        </p>

        {error && <div className="login-error">{error}</div>}

        <form onSubmit={submit}>
          <div className="login-field">
            <label>Server URL</label>
            <input
              value={server}
              onChange={(e) => setServer(e.target.value)}
              placeholder="https://hooks.example.com"
              required
              autoFocus
            />
          </div>
          <div className="login-field">
            <label>Agent ID</label>
            <input
              value={agentId}
              onChange={(e) => setAgentId(e.target.value)}
              placeholder="550e8400-e29b-41d4-a716-446655440000"
              required
            />
          </div>
          <div className="login-field">
            <label>Agent Token</label>
            <input
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="hr_..."
              required
            />
          </div>
          <div className="login-field">
            <label>Agent Name (optional)</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-laptop"
            />
          </div>
          <button type="submit" className="btn btn-primary login-submit" disabled={loading}>
            {loading ? 'Connecting...' : 'Connect'}
          </button>
        </form>

        <div className="login-help">
          <p>The token is stored securely in your OS keychain. It is never sent to the browser after login.</p>
        </div>
      </div>
    </div>
  );
}
