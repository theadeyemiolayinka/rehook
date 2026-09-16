// Public base URL for building inbound webhook URLs. Fetched once from
// /api/settings (REHOOK_PUBLIC_BASE_URL on the server) and cached.
// Falls back to the dashboard origin, which is correct when the dashboard
// is served same-origin behind the public domain.

import { useEffect, useState } from 'react';
import { api } from './api';

interface Settings {
  public_base_url: string;
}

let cached: string | null = null;
let inflight: Promise<string> | null = null;

function fallback(): string {
  return window.location.origin;
}

export function loadPublicBaseUrl(): Promise<string> {
  if (cached) return Promise.resolve(cached);
  if (inflight) return inflight;
  inflight = api
    .get<Settings>('/api/settings/')
    .then((s) => {
      cached = s.public_base_url || fallback();
      return cached;
    })
    .catch(() => fallback())
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function usePublicBaseUrl(): string {
  const [base, setBase] = useState(cached ?? fallback());
  useEffect(() => {
    let alive = true;
    loadPublicBaseUrl().then((b) => {
      if (alive) setBase(b);
    });
    return () => {
      alive = false;
    };
  }, []);
  return base;
}
