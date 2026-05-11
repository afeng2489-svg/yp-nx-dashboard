import { create } from 'zustand';
import { type Node, type Edge, addEdge, applyNodeChanges, applyEdgeChanges } from '@xyflow/react';
import type { Connection, NodeChange, EdgeChange } from '@xyflow/react';
import * as yaml from 'js-yaml';
import { API_BASE_URL } from '@/api/constants';

export type NodeKind =
  | 'agent'
  | 'shell'
  | 'quality_gate'
  | 'condition'
  | 'http'
  | 'approval'
  | 'loop'
  | 'workflow';

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
  updateNodeExecStatus: (
    stageName: string,
    status: NodeData['execStatus'],
    extra?: Partial<NodeData>,
  ) => void;
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

// === Agent prompt templates (for non-AI nodes) ===
// 把 shell/http/workflow 节点翻译成真实可执行的 agent。
// Claude CLI 带 Bash / Fetch 工具，agent prompt 指示它调用相应工具。

function buildAgentFromNode(node: Node<NodeData>, agentId: string): Record<string, unknown> | null {
  const d = node.data;
  switch (d.kind) {
    case 'agent':
      return {
        id: agentId,
        role: d.label,
        model: d.model || 'claude-sonnet-4-6',
        prompt: d.system_prompt || '',
      };
    case 'shell':
      return {
        id: agentId,
        role: '命令执行',
        model: d.model || 'claude-haiku-4-5',
        prompt: [
          '使用 Bash 工具执行以下命令，然后把 stdout、stderr 和退出码完整返回：',
          '',
          `命令: ${d.command || 'echo hello'}`,
          d.timeout ? `超时: ${d.timeout} 秒` : '',
          '',
          '不要解释，直接执行并返回结果。',
        ]
          .filter(Boolean)
          .join('\n'),
      };
    case 'http':
      return {
        id: agentId,
        role: 'HTTP 请求',
        model: d.model || 'claude-haiku-4-5',
        prompt: [
          '使用 Fetch / WebFetch 工具发起 HTTP 请求，返回状态码和响应体：',
          '',
          `方法: ${d.method || 'GET'}`,
          `URL: ${d.url || ''}`,
          '',
          '返回 JSON 格式：{ "status": 状态码, "body": "响应内容" }',
        ].join('\n'),
      };
    case 'workflow':
      return {
        id: agentId,
        role: '子工作流调用',
        model: 'claude-haiku-4-5',
        prompt: [
          '使用 Fetch 工具调用本地 API 启动子工作流并等待结果：',
          '',
          `POST http://localhost:8080/api/v1/workflows/${d.workflowId || ''}/execute`,
          'Body: {"variables": {}}',
          '',
          '返回执行 ID 即可。',
        ].join('\n'),
      };
    default:
      return null; // condition / quality_gate 不生成 agent
  }
}

function nodeToStage(
  node: Node<NodeData>,
  agentId: string | null,
  qualityGate: Record<string, unknown> | null,
): Record<string, unknown> | null {
  const d = node.data;
  const base: Record<string, unknown> = { name: d.label };
  if (qualityGate) base.quality_gate = qualityGate;

  switch (d.kind) {
    case 'agent':
    case 'shell':
    case 'http':
    case 'workflow':
      return { ...base, agents: agentId ? [agentId] : [] };
    case 'approval':
      return {
        ...base,
        stage_type: 'user_input',
        question: d.question || '是否继续？',
        options: (d.options || ['是', '否']).map((v, i) => ({ value: String(i), label: v })),
      };
    case 'loop':
      return {
        ...base,
        stage_type: 'loop',
        loop_var: d.loop_var || 'item',
        max_iterations: d.max_iterations || 10,
      };
    default:
      return null; // condition / quality_gate 融合到其他 stage
  }
}

function stageToNodeData(
  stage: Record<string, unknown>,
  agentsMap: Map<string, Record<string, unknown>>,
): NodeData {
  const stageType = (stage.stage_type as string) || 'agent';
  const agentIds = (stage.agents as string[]) || [];
  const firstAgent = agentIds[0] ? agentsMap.get(agentIds[0]) : undefined;
  const agentRole = (firstAgent?.role as string) || '';

  // 根据 agent role 回推节点 kind
  let kind: NodeKind = 'agent';
  if (stageType === 'user_input') kind = 'approval';
  else if (stageType === 'loop') kind = 'loop';
  else if (agentRole === '命令执行') kind = 'shell';
  else if (agentRole === 'HTTP 请求') kind = 'http';
  else if (agentRole === '子工作流调用') kind = 'workflow';

  return {
    kind,
    label: (stage.name as string) || 'Stage',
    ...KIND_DEFAULTS[kind],
    ...(firstAgent?.model ? { model: firstAgent.model as string } : {}),
    ...(firstAgent?.prompt ? { system_prompt: firstAgent.prompt as string } : {}),
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

    // 1. 为每个"执行型"节点（agent/shell/http/workflow）生成 agent
    const agents: Record<string, unknown>[] = [];
    const nodeToAgentId = new Map<string, string>();
    nodes.forEach((n, i) => {
      const agentId = `${n.data.kind}_${i}`;
      const agent = buildAgentFromNode(n, agentId);
      if (agent) {
        agents.push(agent);
        nodeToAgentId.set(n.id, agentId);
      }
    });

    // 2. 找出 condition / quality_gate 节点（它们会被融合）
    const conditionNodes = nodes.filter((n) => n.data.kind === 'condition');
    const qualityGateNodes = nodes.filter((n) => n.data.kind === 'quality_gate');

    // quality_gate 节点的信息融合到它指向的下游 stage
    const qualityGateByTarget = new Map<string, Record<string, unknown>>();
    qualityGateNodes.forEach((qg) => {
      const outgoing = edges.filter((e) => e.source === qg.id);
      outgoing.forEach((e) => {
        qualityGateByTarget.set(e.target, {
          checks: qg.data.checks || [],
          on_fail: qg.data.on_fail || 'retry',
          max_retries: qg.data.max_retries || 2,
        });
      });
    });

    // 3. 生成 stages（跳过 condition / quality_gate 节点）
    const executableNodes = nodes.filter(
      (n) => n.data.kind !== 'condition' && n.data.kind !== 'quality_gate',
    );

    const stages = executableNodes
      .map((n) => {
        const agentId = nodeToAgentId.get(n.id) ?? null;
        const qg = qualityGateByTarget.get(n.id) || null;
        const stage = nodeToStage(n, agentId, qg);
        if (!stage) return null;

        // 4. 处理 next（包括经过 condition 节点的条件跳转）
        const directNext = edges.filter(
          (e) => e.source === n.id && executableNodes.some((en) => en.id === e.target),
        );
        const conditionNext = edges
          .filter((e) => e.source === n.id && conditionNodes.some((c) => c.id === e.target))
          .flatMap((e) => {
            const condNode = conditionNodes.find((c) => c.id === e.target);
            if (!condNode) return [];
            const condDownstream = edges.filter((ee) => ee.source === condNode.id);
            return condDownstream.map((ee) => {
              const target = nodes.find((nd) => nd.id === ee.target);
              return {
                condition: condNode.data.condition || '',
                goto: target?.data.label || ee.target,
              };
            });
          });

        const nextList: unknown[] = [];
        directNext.forEach((e) => {
          const target = nodes.find((nd) => nd.id === e.target);
          nextList.push({ goto: target?.data.label || e.target });
        });
        conditionNext.forEach((c) => nextList.push(c));
        if (nextList.length > 0) stage.next = nextList;

        return stage;
      })
      .filter((s): s is Record<string, unknown> => s !== null);

    return yaml.dump({ name: workflowName, version: '1.0', agents, stages }, { lineWidth: 120 });
  },

  loadFromYaml: (yamlStr) => {
    try {
      const doc = yaml.load(yamlStr) as Record<string, unknown>;
      const stages = (doc.stages as Record<string, unknown>[]) || [];
      const agentList = (doc.agents as Record<string, unknown>[]) || [];
      const agentsMap = new Map(agentList.map((a) => [a.id as string, a]));

      const newNodes: Node<NodeData>[] = stages.map((stage, i) => ({
        id: `stage-${i}`,
        type: 'custom',
        position: { x: 100 + (i % 4) * 220, y: 100 + Math.floor(i / 4) * 160 },
        data: stageToNodeData(stage, agentsMap),
      }));
      const newEdges: Edge[] = [];
      stages.forEach((stage, i) => {
        const nexts = stage.next as unknown[] | undefined;
        if (!nexts) return;
        nexts.forEach((nx, ni) => {
          const targetName = typeof nx === 'string' ? nx : ((nx as { goto?: string }).goto ?? '');
          const targetIdx = stages.findIndex((s) => s.name === targetName);
          if (targetIdx >= 0) {
            newEdges.push({
              id: `e-${i}-${targetIdx}-${ni}`,
              source: `stage-${i}`,
              target: `stage-${targetIdx}`,
            });
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
        n.data.label === stageName
          ? { ...n, data: { ...n.data, execStatus: status, ...extra } }
          : n,
      ),
    })),

  resetExecStatus: () =>
    set((s) => ({
      nodes: s.nodes.map((n) => ({
        ...n,
        data: {
          ...n.data,
          execStatus: 'idle' as const,
          execDuration: undefined,
          execError: undefined,
        },
      })),
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
      (t) =>
        (t as Record<string, unknown>).type === 'event' &&
        (t as Record<string, unknown>).workflow_ref === downstreamName,
    );
    if (already) return;
    const updated = {
      ...doc,
      triggers: [...triggers, { type: 'event', workflow_ref: downstreamName, pass_output: true }],
    };
    await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ definition: updated }),
    });
  } catch {
    /* best-effort */
  }
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
        (t) =>
          !(
            (t as Record<string, unknown>).type === 'event' &&
            (t as Record<string, unknown>).workflow_ref === downstreamName
          ),
      ),
    };
    await fetch(`${API_BASE_URL}/api/v1/workflows/${upstreamId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ definition: updated }),
    });
  } catch {
    /* best-effort */
  }
}
