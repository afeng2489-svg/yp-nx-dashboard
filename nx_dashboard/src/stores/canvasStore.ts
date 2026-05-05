import { create } from 'zustand';
import { type Node, type Edge, addEdge, applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import type { Connection, NodeChange, EdgeChange } from '@xyflow/react';
import * as yaml from 'js-yaml';
import { API_BASE_URL } from '@/api/constants';

export type NodeKind = 'agent' | 'shell' | 'quality_gate' | 'condition' | 'http' | 'approval' | 'loop' | 'workflow';

export interface NodeData extends Record<string, unknown> {
  kind: NodeKind;
  label: string;
  // workflow node
  workflowId?: string;
  workflowName?: string;
  // agent
  model?: string;
  system_prompt?: string;
  // shell
  command?: string;
  timeout?: number;
  // quality_gate
  checks?: string[];
  on_fail?: string;
  max_retries?: number;
  // condition
  condition?: string;
  // http
  method?: string;
  url?: string;
  // approval
  question?: string;
  options?: string[];
  // loop
  loop_var?: string;
  max_iterations?: number;
  // runtime state
  execStatus?: 'idle' | 'running' | 'success' | 'failed' | 'retrying';
  execDuration?: number;
  execError?: string;
  execTokens?: number;
}

interface CanvasStore {
  nodes: Node<NodeData>[];
  edges: Edge[];
  selectedNodeId: string | null;
  workflowId: string | null;
  workflowName: string;
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  setSelectedNode: (id: string | null) => void;
  updateNodeData: (id: string, data: Partial<NodeData>) => void;
  addNode: (kind: NodeKind, position: { x: number; y: number }) => void;
  setWorkflowId: (id: string | null) => void;
  setWorkflowName: (name: string) => void;
  loadFromYaml: (yamlStr: string) => void;
  toYaml: () => string;
  setNodes: (nodes: Node<NodeData>[]) => void;
  setEdges: (edges: Edge[]) => void;
  updateNodeExecStatus: (stageName: string, status: NodeData['execStatus'], extra?: Partial<NodeData>) => void;
  resetExecStatus: () => void;
}

let nodeCounter = 0;

const KIND_DEFAULTS: Record<NodeKind, Partial<NodeData>> = {
  agent: { model: 'claude-opus-4-7', system_prompt: '' },
  shell: { command: 'echo hello', timeout: 30 },
  quality_gate: { checks: ['cargo test'], on_fail: 'retry', max_retries: 2 },
  condition: { condition: '{{output}} == "ok"' },
  http: { method: 'GET', url: 'https://' },
  approval: { question: '是否继续？', options: ['是', '否'], timeout: 300 },
  loop: { loop_var: 'item', max_iterations: 10 },
  workflow: { workflowId: '', workflowName: '' },
};

const KIND_LABELS: Record<NodeKind, string> = {
  agent: 'AI 调用',
  shell: '代码执行',
  quality_gate: '质量门',
  condition: '条件分支',
  http: 'HTTP 请求',
  approval: '人工审批',
  loop: '循环',
  workflow: '工作流',
};

function nodeToStage(node: Node<NodeData>): Record<string, unknown> {
  const d = node.data;
  const base: Record<string, unknown> = { name: d.label };
  switch (d.kind) {
    case 'agent':
      return { ...base, type: 'agent', model: d.model, system_prompt: d.system_prompt };
    case 'shell':
      return { ...base, type: 'shell', command: d.command, timeout: d.timeout };
    case 'quality_gate':
      return { ...base, type: 'quality_gate', checks: d.checks, on_fail: d.on_fail, max_retries: d.max_retries };
    case 'condition':
      return { ...base, type: 'condition', condition: d.condition };
    case 'http':
      return { ...base, type: 'http', method: d.method, url: d.url };
    case 'approval':
      return { ...base, type: 'user_input', question: d.question, options: d.options, timeout: d.timeout };
    case 'loop':
      return { ...base, type: 'loop', loop_var: d.loop_var, max_iterations: d.max_iterations };
    case 'workflow':
      return { ...base, type: 'workflow', workflowId: d.workflowId, workflowName: d.workflowName };
  }
}

function stageToNodeData(stage: Record<string, unknown>): NodeData {
  const kind = (stage.type as string) === 'user_input' ? 'approval' : (stage.type as NodeKind) || 'agent';
  return {
    kind,
    label: (stage.name as string) || 'Stage',
    ...KIND_DEFAULTS[kind],
    ...(stage as Partial<NodeData>),
  };
}

export const useCanvasStore = create<CanvasStore>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  workflowId: null,
  workflowName: '新工作流',

  onNodesChange: (changes) =>
    set((s) => ({ nodes: applyNodeChanges(changes, s.nodes) as Node<NodeData>[] })),

  onEdgesChange: (changes) => {
    // detect removed edges involving workflow nodes → delete trigger
    const removedIds = changes
      .filter((c) => c.type === 'remove')
      .map((c) => (c as { id: string }).id);
    if (removedIds.length > 0) {
      const { nodes, edges } = get();
      removedIds.forEach((eid) => {
        const edge = edges.find((e) => e.id === eid);
        if (!edge) return;
        const src = nodes.find((n) => n.id === edge.source);
        const tgt = nodes.find((n) => n.id === edge.target);
        if (src?.data.kind === 'workflow' && tgt?.data.kind === 'workflow' && src.data.workflowId) {
          removeWorkflowTrigger(src.data.workflowId as string, tgt.data.workflowName as string);
        }
      });
    }
    set((s) => ({ edges: applyEdgeChanges(changes, s.edges) }));
  },

  onConnect: (connection) => {
    set((s) => {
      const src = s.nodes.find((n) => n.id === connection.source);
      const tgt = s.nodes.find((n) => n.id === connection.target);
      if (src?.data.kind === 'workflow' && tgt?.data.kind === 'workflow' && src.data.workflowId) {
        addWorkflowTrigger(src.data.workflowId as string, tgt.data.workflowName as string);
      }
      return { edges: addEdge(connection, s.edges) };
    });
  },

  setSelectedNode: (id) => set({ selectedNodeId: id }),

  updateNodeData: (id, data) =>
    set((s) => ({
      nodes: s.nodes.map((n) => (n.id === id ? { ...n, data: { ...n.data, ...data } } : n)),
    })),

  addNode: (kind, position) => {
    const id = `${kind}-${++nodeCounter}`;
    const newNode: Node<NodeData> = {
      id,
      type: 'custom',
      position,
      data: { kind, label: `${KIND_LABELS[kind]} ${nodeCounter}`, ...KIND_DEFAULTS[kind] },
    };
    set((s) => ({ nodes: [...s.nodes, newNode] }));
  },

  setWorkflowId: (id) => set({ workflowId: id }),
  setWorkflowName: (name) => set({ workflowName: name }),
  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),

  toYaml: () => {
    const { nodes, edges, workflowName } = get();
    const stages = nodes.map((n) => {
      const stage = nodeToStage(n);
      const nextEdges = edges.filter((e) => e.source === n.id);
      if (nextEdges.length > 0) {
        stage.next = nextEdges.map((e) => {
          const target = nodes.find((nd) => nd.id === e.target);
          return target?.data.label || e.target;
        });
      }
      return stage;
    });
    return yaml.dump({ name: workflowName, version: '1.0', stages }, { lineWidth: 120 });
  },

  loadFromYaml: (yamlStr) => {
    try {
      const doc = yaml.load(yamlStr) as Record<string, unknown>;
      const stages = (doc.stages as Record<string, unknown>[]) || [];
      const newNodes: Node<NodeData>[] = stages.map((stage, i) => ({
        id: `stage-${i}`,
        type: 'custom',
        position: { x: 100 + (i % 4) * 220, y: 100 + Math.floor(i / 4) * 160 },
        data: stageToNodeData(stage),
      }));
      const newEdges: Edge[] = [];
      stages.forEach((stage, i) => {
        const nexts = (stage.next as string[]) || [];
        nexts.forEach((nextName) => {
          const targetIdx = stages.findIndex((s) => s.name === nextName);
          if (targetIdx >= 0) {
            newEdges.push({ id: `e-${i}-${targetIdx}`, source: `stage-${i}`, target: `stage-${targetIdx}` });
          }
        });
      });
      set({
        nodes: newNodes,
        edges: newEdges,
        workflowName: (doc.name as string) || '导入工作流',
      });
    } catch {
      // invalid yaml, ignore
    }
  },

  updateNodeExecStatus: (stageName, status, extra) =>
    set((s) => ({
      nodes: s.nodes.map((n) =>
        n.data.label === stageName ? { ...n, data: { ...n.data, execStatus: status, ...extra } } : n
      ),
    })),

  resetExecStatus: () =>
    set((s) => ({
      nodes: s.nodes.map((n) => ({ ...n, data: { ...n.data, execStatus: 'idle' as const, execDuration: undefined, execError: undefined } })),
    })),
}));

async function addWorkflowTrigger(upstreamId: string, downstreamName: string) {
  try {
    const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`);
    if (!res.ok) return;
    const wf = await res.json();
    const def = wf.data?.definition ?? wf.definition ?? '';
    const doc = (yaml.load(def) as Record<string, unknown>) ?? {};
    const triggers: unknown[] = (doc.triggers as unknown[]) ?? [];
    const already = triggers.some(
      (t) => (t as Record<string, unknown>).type === 'event' && (t as Record<string, unknown>).workflow_ref === downstreamName
    );
    if (already) return;
    const updated = { ...doc, triggers: [...triggers, { type: 'event', workflow_ref: downstreamName, pass_output: true }] };
    await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ definition: updated }),
    });
  } catch { /* best-effort */ }
}

async function removeWorkflowTrigger(upstreamId: string, downstreamName: string) {
  try {
    const res = await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`);
    if (!res.ok) return;
    const wf = await res.json();
    const def = wf.data?.definition ?? wf.definition ?? '';
    const doc = (yaml.load(def) as Record<string, unknown>) ?? {};
    const triggers: unknown[] = (doc.triggers as unknown[]) ?? [];
    const updated = {
      ...doc,
      triggers: triggers.filter(
        (t) => !((t as Record<string, unknown>).type === 'event' && (t as Record<string, unknown>).workflow_ref === downstreamName)
      ),
    };
    await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ definition: updated }),
    });
  } catch { /* best-effort */ }
}
