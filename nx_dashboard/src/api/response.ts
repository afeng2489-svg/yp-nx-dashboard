/**
 * Unwrap API envelope response.
 *
 * Handles both formats:
 * - Envelope: { ok: true, data: T } → T
 * - Envelope error: { ok: false, error: string } → throws
 * - Raw: T (fallback for unmigrated endpoints) → T
 */
export function unwrapEnvelope<T>(body: unknown): T {
  if (body && typeof body === 'object' && 'ok' in body) {
    const envelope = body as { ok: boolean; data?: T; error?: string };
    if (envelope.ok === false) {
      throw new Error(envelope.error ?? 'Unknown error');
    }
    return envelope.data as T;
  }
  return body as T;
}

/**
 * Fetch with timeout and envelope unwrapping.
 */
export async function fetchWithTimeout(
  url: string,
  options: RequestInit = {},
  timeout = 5000,
): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    clearTimeout(timeoutId);
    return response;
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error('Request timeout');
    }
    throw error;
  }
}
