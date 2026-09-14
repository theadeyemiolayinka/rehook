// API client for the HookRelay dashboard. Same-origin in production; proxied
// in dev. Cookie-based sessions, so credentials: 'include'.

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
  const res = await fetch(path, {
    ...init,
    headers,
    body,
    credentials: 'include',
  });
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
export interface User {
  id: string;
  username: string;
  is_admin: boolean;
}

export interface Project {
  id: string;
  name: string;
  slug: string;
  description: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface Endpoint {
  id: string;
  project_id: string;
  name: string;
  public_identifier: string;
  enabled: boolean;
  provider: string | null;
  created_at: string;
  updated_at: string;
}

export interface EventListRow {
  id: string;
  project_id: string;
  endpoint_id: string;
  request_method: string;
  content_type: string | null;
  remote_address: string | null;
  received_at: string;
  payload_size: number;
  delivery_state: string;
}

export interface EventDetail {
  id: string;
  project_id: string;
  endpoint_id: string;
  request_method: string;
  content_type: string | null;
  remote_address: string | null;
  received_at: string;
  payload_size: number;
  delivery_state: string;
  headers: Record<string, string | string[]>;
  body: string | null; // base64
}
