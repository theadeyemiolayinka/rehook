import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { api, type User } from '../lib/api';
import { Button } from '../components/Button';
import { Input } from '../components/Input';
import { Logo } from '../components/Logo';
import { useToast } from '../components/Toast';
import './Login.css';

export function Login() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const navigate = useNavigate();
  const toast = useToast();

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await api.post<{ user: User }>('/api/auth/login', {
        username,
        password,
      });
      toast.show(`Signed in as ${res.user.username}`, 'success');
      navigate('/');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Sign in failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="login-page">
      <form className="login-card" onSubmit={handleSubmit}>
        <div className="login-brand">
          <Logo size={28} withWordmark />
        </div>
        <div className="login-heading">
          <h1>Sign in</h1>
          <p>Server administration dashboard.</p>
        </div>
        <Input
          label="Username"
          name="username"
          autoComplete="username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoFocus
          required
        />
        <Input
          label="Password"
          name="password"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          error={error || undefined}
          required
        />
        <Button
          type="submit"
          variant="primary"
          loading={loading}
          className="login-submit"
        >
          Sign in
        </Button>
      </form>
    </div>
  );
}
