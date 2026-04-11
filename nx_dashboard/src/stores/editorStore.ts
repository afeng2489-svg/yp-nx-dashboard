import { create } from 'zustand';
import {
  Node,
  Edge,
  Connection,
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  NodeChange,
  EdgeChange,
} from '@xyflow/react';

const generateId = (): string =>
  `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;

export type NodeType = 'agent' | 'stage' | 'condition' | 'loop';
export type CliProvider = 'claude' | 'ollama' | 'custom';

export interface AgentConfig {
  role: string;
  model: string;
  prompt: string;
  cliProvider: CliProvider;
}

export interface StageConfig {
  name: string;
  parallel: boolean;
  agents: string[];
}

export interface ConditionConfig {
  expression: string;
  trueLabel: string;
  falseLabel: string;
}

export interface LoopConfig {
  maxIterations: number;
  continueOnError: boolean;
}

export type NodeData = {
  type: NodeType;
  label: string;
  config: AgentConfig | StageConfig | ConditionConfig | LoopConfig;
};

interface EditorStore {
  nodes: Node<NodeData>[];
  edges: Edge[];
  selectedNodeId: string | null;
  isCommandPaletteOpen: boolean;
  isDirty: boolean;
  workflowName: string;
  loadedWorkflowId: string | null;

  onNodesChange: (changes: NodeChange<Node<NodeData>>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (type: NodeType, position: { x: number; y: number }) => void;
  updateNodeData: (nodeId: string, data: Partial<NodeData>) => void;
  deleteNode: (nodeId: string) => void;
  selectNode: (nodeId: string | null) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setWorkflowName: (name: string) => void;
  loadTemplate: (template: { nodes: Node<NodeData>[]; edges: Edge[] }) => void;
  loadWorkflow: (workflow: { id: string; name: string; stages?: { name: string; agents: string[]; parallel: boolean }[]; agents?: { id: string; role: string; model: string; prompt: string; depends_on: string[] }[] }) => void;
  clearCanvas: () => void;
  exportWorkflow: () => { nodes: Node<NodeData>[]; edges: Edge[]; name: string; id?: string };
}

const defaultAgentConfig: AgentConfig = {
  role: 'developer',
  model: 'claude-opus-4-6',
  prompt: 'You are a helpful assistant.',
  cliProvider: 'claude',
};

const defaultStageConfig: StageConfig = {
  name: 'New Stage',
  parallel: false,
  agents: [],
};

const defaultConditionConfig: ConditionConfig = {
  expression: '',
  trueLabel: 'Yes',
  falseLabel: 'No',
};

const defaultLoopConfig: LoopConfig = {
  maxIterations: 10,
  continueOnError: false,
};

const getDefaultConfig = (type: NodeType) => {
  switch (type) {
    case 'agent':
      return { ...defaultAgentConfig, prompt: `You are a helpful ${defaultAgentConfig.role}.` };
    case 'stage':
      return defaultStageConfig;
    case 'condition':
      return defaultConditionConfig;
    case 'loop':
      return defaultLoopConfig;
  }
};

const getDefaultLabel = (type: NodeType): string => {
  switch (type) {
    case 'agent':
      return 'New Agent';
    case 'stage':
      return 'New Stage';
    case 'condition':
      return 'Condition';
    case 'loop':
      return 'Loop';
  }
};

export const useEditorStore = create<EditorStore>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNodeId: null,
  isCommandPaletteOpen: false,
  isDirty: false,
  workflowName: 'Untitled Workflow',
  loadedWorkflowId: null,

  onNodesChange: (changes) => {
    set((state) => ({
      nodes: applyNodeChanges(changes, state.nodes) as Node<NodeData>[],
      isDirty: true,
    }));
  },

  onEdgesChange: (changes) => {
    set((state) => ({
      edges: applyEdgeChanges(changes, state.edges),
      isDirty: true,
    }));
  },

  onConnect: (connection) => {
    set((state) => ({
      edges: addEdge(
        {
          ...connection,
          id: generateId(),
          animated: true,
          style: { stroke: 'hsl(var(--primary))' },
        },
        state.edges
      ),
      isDirty: true,
    }));
  },

  addNode: (type, position) => {
    const newNode: Node<NodeData> = {
      id: generateId(),
      type: 'workflowNode',
      position,
      data: {
        type,
        label: getDefaultLabel(type),
        config: getDefaultConfig(type),
      },
    };
    set((state) => ({
      nodes: [...state.nodes, newNode],
      isDirty: true,
    }));
  },

  updateNodeData: (nodeId, data) => {
    set((state) => ({
      nodes: state.nodes.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, ...data } }
          : node
      ),
      isDirty: true,
    }));
  },

  deleteNode: (nodeId) => {
    set((state) => ({
      nodes: state.nodes.filter((node) => node.id !== nodeId),
      edges: state.edges.filter(
        (edge) => edge.source !== nodeId && edge.target !== nodeId
      ),
      selectedNodeId: state.selectedNodeId === nodeId ? null : state.selectedNodeId,
      isDirty: true,
    }));
  },

  selectNode: (nodeId) => {
    set({ selectedNodeId: nodeId });
  },

  setCommandPaletteOpen: (open) => {
    set({ isCommandPaletteOpen: open });
  },

  setWorkflowName: (name) => {
    set({ workflowName: name, isDirty: true });
  },

  loadTemplate: (template) => {
    set({
      nodes: template.nodes,
      edges: template.edges,
      isDirty: true,
      selectedNodeId: null,
      loadedWorkflowId: null,
    });
  },

  loadWorkflow: (workflow) => {
    // Convert workflow format to editor nodes
    const nodes: Node<NodeData>[] = [];
    const edges: Edge[] = [];
    const agentIdToNodeId: Record<string, string> = {};

    if (workflow.stages && workflow.agents) {
      // Create stage nodes and agent nodes
      workflow.stages.forEach((stage, stageIndex) => {
        const stageNodeId = `stage-${stageIndex}`;
        nodes.push({
          id: stageNodeId,
          type: 'workflowNode',
          position: { x: 400, y: stageIndex * 200 },
          data: {
            type: 'stage' as NodeType,
            label: stage.name,
            config: {
              name: stage.name,
              parallel: stage.parallel,
              agents: stage.agents,
            },
          },
        });

        // Create agent nodes for this stage
        stage.agents.forEach((agentRole, agentIndex) => {
          const agent = workflow.agents?.find(a => a.role === agentRole);
          if (agent) {
            const agentNodeId = agent.id;
            agentIdToNodeId[agent.id] = agentNodeId;
            nodes.push({
              id: agentNodeId,
              type: 'workflowNode',
              position: { x: 200 + agentIndex * 200, y: stageIndex * 200 + 100 },
              data: {
                type: 'agent' as NodeType,
                label: agent.role.charAt(0).toUpperCase() + agent.role.slice(1),
                config: {
                  role: agent.role,
                  model: agent.model,
                  prompt: agent.prompt,
                  cliProvider: 'claude' as CliProvider,
                },
              },
            });

            // Connect stage to agent
            edges.push({
              id: `${stageNodeId}-${agentNodeId}`,
              source: stageNodeId,
              target: agentNodeId,
              animated: true,
              style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
            });
          }
        });
      });

      // Create dependency edges
      workflow.agents.forEach(agent => {
        agent.depends_on.forEach(depId => {
          const depNodeId = agentIdToNodeId[depId];
          const agentNodeId = agentIdToNodeId[agent.id];
          if (depNodeId && agentNodeId) {
            edges.push({
              id: `${depNodeId}-${agentNodeId}`,
              source: depNodeId,
              target: agentNodeId,
              animated: true,
              style: { stroke: 'hsl(142 76% 36%)' },
            });
          }
        });
      });
    }

    set({
      nodes,
      edges,
      workflowName: workflow.name,
      loadedWorkflowId: workflow.id,
      isDirty: false,
      selectedNodeId: null,
    });
  },

  clearCanvas: () => {
    set({
      nodes: [],
      edges: [],
      isDirty: false,
      selectedNodeId: null,
      workflowName: 'Untitled Workflow',
      loadedWorkflowId: null,
    });
  },

  exportWorkflow: () => {
    const { nodes, edges, workflowName, loadedWorkflowId } = get();
    return { nodes, edges, name: workflowName, id: loadedWorkflowId || undefined };
  },
}));