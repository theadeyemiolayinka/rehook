import { useEffect, useState } from 'react';
import { BrowserRouter, Routes as RouterRoutes, Route, Navigate, useParams } from 'react-router-dom';
import { Layout } from './components/Layout';
import { Loading } from './components/Loading';
import { ToastProvider } from './components/Toast';
import { Connection } from './pages/Connection';
import { Login } from './pages/Login';
import { Targets } from './pages/Targets';
import { Routes } from './pages/Routes';
import { Events } from './pages/Events';
import { EventDetail } from './pages/EventDetail';
import { History } from './pages/History';
import { Settings } from './pages/Settings';
import { api, type ConnectionStatus } from './lib/api';

function EventDetailWrapper() {
  const { id } = useParams<{ id: string }>();
  if (!id) return <Navigate to="/events" replace />;
  return <EventDetail id={id} />;
}

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
      <ToastProvider>
        <BrowserRouter>
          <RouterRoutes>
            <Route path="*" element={<Login onLoggedIn={loadStatus} />} />
          </RouterRoutes>
        </BrowserRouter>
      </ToastProvider>
    );
  }

  return (
    <ToastProvider>
      <BrowserRouter>
        <Layout>
          <RouterRoutes>
            <Route
              path="/"
              element={<Connection status={status} onRefresh={loadStatus} />}
            />
            <Route path="/targets" element={<Targets />} />
            <Route path="/routes" element={<Routes />} />
            <Route path="/events" element={<Events />} />
            <Route path="/events/:id" element={<EventDetailWrapper />} />
            <Route path="/history" element={<History />} />
            <Route
              path="/settings"
              element={<Settings onLogout={loadStatus} />}
            />
            <Route path="*" element={<Navigate to="/" replace />} />
          </RouterRoutes>
        </Layout>
      </BrowserRouter>
    </ToastProvider>
  );
}
