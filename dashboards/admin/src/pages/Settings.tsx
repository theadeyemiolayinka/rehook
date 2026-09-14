import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';

export function Settings() {
  return (
    <div>
      <PageHeader title="Settings" subtitle="Server configuration." />
      <EmptyState
        title="No configurable settings yet"
        description="Retention, payload limits, and delivery behavior will be configurable here in a later milestone."
      />
    </div>
  );
}
