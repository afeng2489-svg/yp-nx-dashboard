import type { Execution } from '@/stores/executionStore';

/** Extract preview session_id from execution stage outputs or variables. */
export function extractPreviewSessionId(execution: Execution): string | null {
  const fromVars = execution.variables?.session_id ?? execution.variables?.preview_session_id;
  if (typeof fromVars === 'string' && fromVars.length > 0) {
    return fromVars;
  }

  for (const stage of execution.stage_results ?? []) {
    if (!stage.outputs?.length) continue;
    for (const output of stage.outputs) {
      const parsed = tryParseJson(output.content);
      if (parsed) {
        const sid = parsed.session_id ?? parsed.preview_session_id;
        if (typeof sid === 'string' && sid.length > 0) return sid;
      }
      if (output.content) {
        const match = output.content.match(/"session_id"\s*:\s*"([^"]+)"/);
        if (match?.[1]) return match[1];
      }
    }
  }

  return null;
}

function tryParseJson(content?: string): Record<string, unknown> | null {
  if (!content?.trim()) return null;
  try {
    const value = JSON.parse(content);
    return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : null;
  } catch {
    const jsonMatch = content.match(/\{[\s\S]*\}/);
    if (!jsonMatch) return null;
    try {
      const value = JSON.parse(jsonMatch[0]);
      return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : null;
    } catch {
      return null;
    }
  }
}
