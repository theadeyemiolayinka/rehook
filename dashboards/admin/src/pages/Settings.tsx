import { useCallback, useEffect, useState } from 'react';
import { api, ApiError } from '../lib/api';
import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import { Loading } from '../components/Loading';
import { Button } from '../components/Button';
import './Settings.css';

interface ServerSettings {
  public_base_url: string;
  max_webhook_body_kb: number;
  max_stored_events: number;
  event_retention_days: number;
  session_ttl_hours: number;
  trusted_proxy_hops: number;
  version: string;
}

export function Settings() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [settings, setSettings] = useState<ServerSettings | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const s = await api.get<ServerSettings>('/api/settings');
      setSettings(s);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'Failed to load settings');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) {
    return (
      <div className="settings">
        <PageHeader title="Settings" subtitle="Server configuration." />
        <div className="settings-loading">
          <Loading />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="settings">
        <PageHeader title="Settings" subtitle="Server configuration." />
        <EmptyState
          title="Unable to load settings"
          description={error}
          action={<Button onClick={load}>Retry</Button>}
        />
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="settings">
        <PageHeader title="Settings" subtitle="Server configuration." />
        <EmptyState title="No settings available" />
      </div>
    );
  }

  return (
    <div className="settings">
      <PageHeader
        title="Settings"
        subtitle="Server configuration. Values are set through environment variables."
      />

      <section className="settings-section">
        <h2 className="settings-section-title">Server</h2>
        <dl className="settings-list">
          <div className="settings-row">
            <dt>Public base URL</dt>
            <dd className="settings-mono">{settings.public_base_url}</dd>
          </div>
          <div className="settings-row">
            <dt>Server version</dt>
            <dd className="settings-mono">{settings.version}</dd>
          </div>
          <div className="settings-row">
            <dt>Trusted proxy hops</dt>
            <dd>{settings.trusted_proxy_hops}</dd>
          </div>
        </dl>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Webhooks</h2>
        <dl className="settings-list">
          <div className="settings-row">
            <dt>Max body size</dt>
            <dd>{settings.max_webhook_body_kb} KB</dd>
          </div>
          <div className="settings-row">
            <dt>Max stored events</dt>
            <dd>{settings.max_stored_events.toLocaleString()}</dd>
          </div>
          <div className="settings-row">
            <dt>Event retention</dt>
            <dd>{settings.event_retention_days} days</dd>
          </div>
        </dl>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Sessions</h2>
        <dl className="settings-list">
          <div className="settings-row">
            <dt>Session TTL</dt>
            <dd>{settings.session_ttl_hours} hours</dd>
          </div>
        </dl>
      </section>

      <p className="settings-help">
        These values are read-only in the dashboard. Change them by updating the
        corresponding environment variables and restarting the server.
      </p>
    </div>
  );
}
