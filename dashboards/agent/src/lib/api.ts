// API client for the Rehook agent local API. Same-origin in production
// (served by the agent itself); proxied in dev.

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(
  path: string,
  init?: RequestInit & { json?: unknown },
): Promise<T> {
  const headers: Record<string, string> = { ...(init?.headers as object) };
  let body = init?.body;
  if (init?.json !== undefined) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(init.json);
  }
  const res = await fetch(path, { ...init, headers, body });
  const text = await res.text();
  let data: unknown = null;
  if (text) {
    try {
      data = JSON.parse(text);
    } catch {
      data = text;
    }
  }
  if (!res.ok) {
    const message =
      typeof data === 'object' && data && 'error' in data
        ? String((data as { error: string }).error)
        : res.statusText;
    throw new ApiError(res.status, message);
  }
  return data as T;
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, json?: unknown) =>
    request<T>(path, { method: 'POST', json }),
  patch: <T>(path: string, json?: unknown) =>
    request<T>(path, { method: 'PATCH', json }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};

// Types

export interface ConnectionStatus {
  server_url: string | null;
  agent_id: string | null;
  agent_name: string | null;
  authenticated: boolean;
  connected: boolean;
  last_connected_at: string | null;
  subscribed_endpoints: string[];
}

export interface Target {
  id: string;
  url: string;
}

export interface RouteEntry {
  endpoint_id: string;
  target_id: string;
}

export interface ServerEndpoint {
  id: string;
  name: string;
}

export interface ServerProject {
  id: string;
  name: string;
  slug: string;
  endpoints: ServerEndpoint[];
}

export interface DeliveryRecord {
  id: string;
  event_id: string;
  target_id: string;
  attempt_number: number;
  status: string;
  http_status: number | null;
  duration_ms: number | null;
  error_category: string | null;
  error_message: string | null;
  response_headers: Record<string, string> | null;
  response_body: string | null; // base64
  started_at: string;
  completed_at: string | null;
}

export interface StoredEvent {
  id: string;
  project_id: string;
  endpoint_id: string;
  request_method: string;
  content_type: string | null;
  received_at: string;
  payload_size: number;
  headers: Record<string, string | string[]>;
  body: string | null;
}

export interface DbStats {
  deliveries_count: number;
  events_count: number;
  db_size_bytes: number;
}

export interface LoginRequest {
  server: string;
  agent_id: string;
  token: string;
  name?: string;
}

export interface LoginResponse {
  ok: boolean;
  agent_id: string;
  server: string;
}
