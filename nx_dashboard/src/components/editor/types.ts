import { Node, Edge } from '@xyflow/react';
import { AgentConfig, StageConfig, ConditionConfig, LoopConfig, NodeType } from '@/stores/editorStore';

export type { NodeType, AgentConfig, StageConfig, ConditionConfig, LoopConfig };

export interface WorkflowNodeData extends Record<string, unknown> {
  type: NodeType;
  label: string;
  config: AgentConfig | StageConfig | ConditionConfig | LoopConfig;
}

export type WorkflowNode = Node<WorkflowNodeData>;

export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  category: 'basic' | 'collaboration' | 'testing' | 'brainstorm' | 'planning' | 'development';
  nodes: WorkflowNode[];
  edges: Edge[];
}

export interface CommandAction {
  id: string;
  label: string;
  shortcut?: string;
  icon: string;
  action: () => void;
}

export const NODE_COLORS: Record<NodeType, string> = {
  agent: 'hsl(221.2 83.2% 53.3%)',
  stage: 'hsl(142 76% 36%)',
  condition: 'hsl(38 92% 50%)',
  loop: 'hsl(280 65% 60%)',
};

export const NODE_ICONS: Record<NodeType, string> = {
  agent: '👤',
  stage: '📦',
  condition: '🔀',
  loop: '🔄',
};

export const AGENT_ROLES = [
  { value: 'developer', label: 'Developer', description: 'Writes and reviews code' },
  { value: 'reviewer', label: 'Reviewer', description: 'Reviews code and provides feedback' },
  { value: 'tester', label: 'Tester', description: 'Writes and runs tests' },
  { value: 'planner', label: 'Planner', description: 'Creates plans and break down tasks' },
  { value: 'researcher', label: 'Researcher', description: 'Researches and gathers information' },
  { value: 'writer', label: 'Writer', description: 'Writes documentation and content' },
  { value: 'coordinator', label: 'Coordinator', description: 'Orchestrates other agents' },
] as const;

export const CLI_PROVIDERS = [
  { value: 'claude', label: 'Claude CLI', description: ' Anthropic Claude via CLI' },
  { value: 'ollama', label: 'Ollama', description: 'Local LLM server' },
  { value: 'custom', label: 'Custom', description: 'Custom CLI endpoint' },
] as const;

export const MODEL_OPTIONS = [
  { value: 'claude-opus-4-6', label: 'Claude Opus 4.6', description: 'Most capable model' },
  { value: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6', description: 'Balanced performance' },
  { value: 'claude-haiku-4-5', label: 'Claude Haiku 4.5', description: 'Fast and efficient' },
  { value: 'gpt-4o', label: 'GPT-4o', description: 'OpenAI model' },
  { value: 'gpt-4o-mini', label: 'GPT-4o Mini', description: 'Fast OpenAI model' },
] as const;