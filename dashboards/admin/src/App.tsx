import { useEffect, useState } from 'react';
import {
  BrowserRouter,
  Routes,
  Route,
  Navigate,
  useLocation,
} from 'react-router-dom';
import { api } from './lib/api';
import { ToastProvider } from './components/Toast';
import { Layout } from './components/Layout';
import { Loading } from './components/Loading';
import { Login } from './pages/Login';
import { Overview } from './pages/Overview';
import { Projects } from './pages/Projects';
import { ProjectDetail } from './pages/ProjectDetail';
import { Endpoints } from './pages/Endpoints';
import { Events } from './pages/Events';
import { EventDetail } from './pages/EventDetail';
import { Agents } from './pages/Agents';
import { Settings } from './pages/Settings';

function useAuth() {
  const [state, setState] = useState<'loading' | 'authed' | 'unauthed'>('loading');

  useEffect(() => {
    let cancelled = false;
    api
      .get('/api/auth/me')
      .then(() => {
        if (!cancelled) setState('authed');
      })
      .catch(() => {
        if (!cancelled) setState('unauthed');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return state;
}

function ProtectedRoutes() {
  const auth = useAuth();
  const location = useLocation();

  if (auth === 'loading') {
    return (
      <div style={{ padding: 64 }}>
        <Loading />
      </div>
    );
  }
  if (auth === 'unauthed') {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<Overview />} />
        <Route path="/projects" element={<Projects />} />
        <Route path="/projects/:id" element={<ProjectDetail />} />
        <Route path="/endpoints" element={<Endpoints />} />
        <Route path="/events" element={<Events />} />
        <Route path="/events/:id" element={<EventDetail />} />
        <Route path="/agents" element={<Agents />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Layout>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route path="/*" element={<ProtectedRoutes />} />
        </Routes>
      </BrowserRouter>
    </ToastProvider>
  );
}
