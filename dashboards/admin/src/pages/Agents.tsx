import { PageHeader } from '../components/PageHeader';
import { EmptyState } from '../components/EmptyState';
import './Agents.css';

export function Agents() {
  return (
    <div className="agents">
      <PageHeader
        title="Agents"
        subtitle="Connected local agents that receive delivery instructions."
      />
      <EmptyState
        title="No agents connected"
        description="Run the HookRelay agent on your machine and sign in to connect it. Agents are listed here once they connect."
      />
    </div>
  );
}
