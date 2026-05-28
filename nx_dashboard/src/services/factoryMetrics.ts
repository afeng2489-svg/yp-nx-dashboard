import { API_BASE_URL } from '@/api/constants';
import { writebackSprintOnRunComplete } from './sprintWriteback';

const DEVICE_KEY = 'nexus-factory-device-id';
const RUN_META_PREFIX = 'factory-run-meta:';

export type FactoryEventType =
  | 'factory_opened'
  | 'run_started'
  | 'first_artifact'
  | 'run_completed'
  | 'terminal_fallback';

export interface RunMeta {
  golden_path: boolean;
  started_at: number;
  first_artifact_recorded?: boolean;
  /** AF-08 Sprint 卡片关联 */
  sprint_id?: string;
}

export interface FactoryMetrics {
  activation: { rate: number; numerator: number; denominator: number };
  golden_path_success: { rate: number; numerator: number; denominator: number };
  time_to_first_diff: { median_minutes: number; samples: number };
  run_completion: { rate: number; numerator: number; denominator: number };
  terminal_fallback: { rate: number; numerator: number; denominator: number };
  w2_retention: { rate: number; numerator: number; denominator: number };
}

function deviceId(): string {
  try {
    let id = localStorage.getItem(DEVICE_KEY);
    if (!id) {
      id =
        typeof crypto !== 'undefined' && 'randomUUID' in crypto
          ? crypto.randomUUID()
          : `dev-${Date.now()}`;
      localStorage.setItem(DEVICE_KEY, id);
    }
    return id;
  } catch {
    return 'anonymous';
  }
}

export function saveRunMeta(executionId: string, meta: RunMeta) {
  try {
    sessionStorage.setItem(`${RUN_META_PREFIX}${executionId}`, JSON.stringify(meta));
  } catch {
    /* ignore */
  }
}

export function getRunMeta(executionId: string): RunMeta | null {
  try {
    const raw = sessionStorage.getItem(`${RUN_META_PREFIX}${executionId}`);
    return raw ? (JSON.parse(raw) as RunMeta) : null;
  } catch {
    return null;
  }
}

export function clearRunMeta(executionId: string) {
  try {
    sessionStorage.removeItem(`${RUN_META_PREFIX}${executionId}`);
  } catch {
    /* ignore */
  }
}

export async function recordFactoryEvent(
  eventType: FactoryEventType,
  opts: { executionId?: string; payload?: Record<string, unknown> } = {},
) {
  try {
    await fetch(`${API_BASE_URL}/api/v1/factory/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        device_id: deviceId(),
        event_type: eventType,
        execution_id: opts.executionId,
        payload: opts.payload ?? {},
      }),
    });
  } catch {
    /* best-effort local metrics */
  }
}

export async function fetchFactoryMetrics(): Promise<FactoryMetrics | null> {
  try {
    const res = await fetch(`${API_BASE_URL}/api/v1/factory/metrics`);
    if (!res.ok) return null;
    return (await res.json()) as FactoryMetrics;
  } catch {
    return null;
  }
}

export async function maybeRecordFirstArtifact(executionId: string) {
  const meta = getRunMeta(executionId);
  if (!meta || meta.first_artifact_recorded) return;
  meta.first_artifact_recorded = true;
  saveRunMeta(executionId, meta);
  await recordFactoryEvent('first_artifact', {
    executionId,
    payload: {
      golden_path: meta.golden_path,
      elapsed_ms: Math.max(0, Date.now() - meta.started_at),
    },
  });
}

export async function recordRunCompleted(
  executionId: string,
  status: 'completed' | 'failed' | 'cancelled',
) {
  const meta = getRunMeta(executionId);
  if (meta?.sprint_id) {
    void writebackSprintOnRunComplete(meta.sprint_id, status);
  }
  await recordFactoryEvent('run_completed', {
    executionId,
    payload: {
      status,
      golden_path: meta?.golden_path ?? false,
    },
  });
  clearRunMeta(executionId);
}

export async function recordTerminalFallback(executionId?: string) {
  await recordFactoryEvent('terminal_fallback', {
    executionId,
    payload: {},
  });
}

/** Golden Path 预填任务（与 factoryQuickStart 一致） */
export const GOLDEN_PATH_TASK = '给 README.md 增加「快速开始」安装步骤';

export function isGoldenPathPrompt(prompt: string): boolean {
  const n = prompt.trim();
  return n === GOLDEN_PATH_TASK || /readme.*快速开始/i.test(n);
}
