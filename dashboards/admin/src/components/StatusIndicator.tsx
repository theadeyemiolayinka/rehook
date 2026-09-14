import './StatusIndicator.css';

type State = 'connected' | 'disconnected' | 'delivered' | 'failed' | 'pending' | 'disabled' | 'receiving' | 'enabled';

interface Props {
  state: State;
  label?: string;
}

const LABELS: Record<State, string> = {
  connected: 'Connected',
  disconnected: 'Disconnected',
  delivered: 'Delivered',
  failed: 'Failed',
  pending: 'Pending',
  disabled: 'Disabled',
  receiving: 'Receiving',
  enabled: 'Enabled',
};

export function StatusIndicator({ state, label }: Props) {
  return (
    <span className={`status status-${state}`}>
      <span className="status-dot" />
      {label ?? LABELS[state]}
    </span>
  );
}
