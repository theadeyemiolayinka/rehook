import { useEffect, useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Loading } from './components/Loading';
import { Connection } from './pages/Connection';
import { Login } from './pages/Login';
import { Targets } from './pages/Targets';
import { Events } from './pages/Events';
import { History } from './pages/History';
import { Settings } from './pages/Settings';
import { api, type ConnectionStatus } from './lib/api';

export default function App() {
  const [status, setStatus] = useState<ConnectionStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const loadStatus = async () => {
    try {
      const s = await api.get<ConnectionStatus>('/api/status');
      setStatus(s);
    } catch {
      setStatus(null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStatus();
  }, []);

  if (loading) {
    return (
      <div style={{ padding: 64 }}>
        <Loading />
      </div>
    );
  }

  const authenticated = status?.authenticated ?? false;

  if (!authenticated) {
    return (
      <BrowserRouter>
        <Routes>
          <Route path="*" element={<Login onLoggedIn={loadStatus} />} />
        </Routes>
      </BrowserRouter>
    );
  }

  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Connection status={status} onRefresh={loadStatus} />} />
          <Route path="/targets" element={<Targets />} />
          <Route path="/events" element={<Events />} />
          <Route path="/history" element={<History />} />
          <Route path="/settings" element={<Settings onLogout={loadStatus} />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}
