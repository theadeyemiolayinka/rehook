import './Loading.css';

export function Loading() {
  return (
    <div className="loading">
      <span className="loading-spinner" />
    </div>
  );
}

export function ErrorState({ message }: { message: string }) {
  return <div className="error-state">{message}</div>;
}
