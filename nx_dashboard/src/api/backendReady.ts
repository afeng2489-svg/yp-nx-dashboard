import { API_BASE_URL } from './constants';

const HEALTH_URL = `${API_BASE_URL}/health`;
const MAX_RETRIES = 30;
const INITIAL_DELAY_MS = 500;
const MAX_DELAY_MS = 3000;

let backendReady = false;
let waitingPromise: Promise<boolean> | null = null;

/**
 * Poll /health until the backend responds or we exhaust retries.
 * Returns true if backend is ready, false if it never responded.
 *
 * Call this once at app startup; subsequent calls return immediately.
 */
export async function waitForBackend(): Promise<boolean> {
  if (backendReady) return true;
  if (waitingPromise) return waitingPromise;

  waitingPromise = pollHealth();
  const result = await waitingPromise;
  backendReady = result;
  waitingPromise = null;
  return result;
}

async function pollHealth(): Promise<boolean> {
  let delay = INITIAL_DELAY_MS;

  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 2000);

      const res = await fetch(HEALTH_URL, {
        signal: controller.signal,
        cache: 'no-store',
      });
      clearTimeout(timeout);

      if (res.ok) {
        const ct = res.headers.get('content-type') ?? '';
        if (ct.includes('application/json')) {
          const body = (await res.json()) as { ok?: boolean; data?: { status?: string } };
          if (body.ok === true || body.data?.status === 'ok') {
            console.log(`[NX Dashboard] Backend ready (attempt ${attempt})`);
            return true;
          }
        }
      }
    } catch {
      // Connection refused or timeout — backend not ready yet
    }

    await new Promise((r) => setTimeout(r, delay));
    delay = Math.min(delay * 1.3, MAX_DELAY_MS);
  }

  console.error('[NX Dashboard] Backend not ready after max retries');
  return false;
}

/** Reset state for testing */
export function _resetBackendReady(): void {
  backendReady = false;
  waitingPromise = null;
}
