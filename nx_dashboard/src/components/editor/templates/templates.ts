import { Node } from '@xyflow/react';
import { WorkflowTemplate, WorkflowNodeData } from '../types';

const generateId = (): string =>
  `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;

const createAgentNode = (
  role: string,
  model: string,
  x: number,
  y: number,
  prompt?: string
): Node<WorkflowNodeData> => ({
  id: generateId(),
  type: 'workflowNode',
  position: { x, y },
  data: {
    type: 'agent',
    label: role.charAt(0).toUpperCase() + role.slice(1),
    config: {
      role,
      model,
      prompt: prompt || `You are a helpful ${role}.`,
      cliProvider: 'claude',
    },
  },
});

const createStageNode = (
  name: string,
  x: number,
  y: number,
  parallel: boolean = false
): Node<WorkflowNodeData> => ({
  id: generateId(),
  type: 'workflowNode',
  position: { x, y },
  data: {
    type: 'stage',
    label: name,
    config: {
      name,
      parallel,
      agents: [],
    },
  },
});

export const workflowTemplates: WorkflowTemplate[] = [
  {
    id: 'simple-plan',
    name: 'Simple Plan',
    description: 'Single agent task execution',
    category: 'basic',
    nodes: [
      createAgentNode('planner', 'claude-opus-4-6', 400, 200),
    ],
    edges: [],
  },
  {
    id: 'multi-cli-plan',
    name: 'Multi-CLI Plan',
    description: 'Parallel execution across multiple agents',
    category: 'collaboration',
    nodes: [
      createStageNode('Parallel Tasks', 400, 100, true),
      createAgentNode('developer', 'claude-sonnet-4-6', 250, 250, 'You are a developer agent that writes code.'),
      createAgentNode('reviewer', 'claude-sonnet-4-6', 400, 250, 'You are a reviewer agent that reviews code.'),
      createAgentNode('tester', 'claude-sonnet-4-6', 550, 250, 'You are a tester agent that runs tests.'),
    ],
    edges: [
      {
        id: generateId(),
        source: '0',
        target: '1',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
      {
        id: generateId(),
        source: '0',
        target: '2',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
      {
        id: generateId(),
        source: '0',
        target: '3',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
    ],
  },
  {
    id: 'tdd-workflow',
    name: 'TDD Workflow',
    description: 'Test-driven development cycle',
    category: 'testing',
    nodes: [
      createStageNode('Write Test', 300, 100, false),
      createAgentNode('tester', 'claude-haiku-4-5', 150, 220, 'Write a failing test for the next feature.'),
      createAgentNode('developer', 'claude-opus-4-6', 300, 220, 'Implement the feature to make the test pass.'),
      createAgentNode('reviewer', 'claude-sonnet-4-6', 450, 220, 'Review the implementation for quality.'),
    ],
    edges: [
      {
        id: generateId(),
        source: '0',
        target: '1',
        animated: true,
        style: { stroke: 'hsl(142 76% 36%)' },
      },
      {
        id: generateId(),
        source: '1',
        target: '2',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
      {
        id: generateId(),
        source: '2',
        target: '3',
        animated: true,
        style: { stroke: 'hsl(38 92% 50%)' },
      },
    ],
  },
  {
    id: 'brainstorm',
    name: 'Brainstorm',
    description: 'Multi-role brainstorming session',
    category: 'brainstorm',
    nodes: [
      createStageNode('Brainstorm', 400, 80, true),
      createAgentNode('planner', 'claude-opus-4-6', 200, 200, 'You are a planner. Break down the problem into actionable steps.'),
      createAgentNode('researcher', 'claude-sonnet-4-6', 330, 200, 'You are a researcher. Gather relevant information and context.'),
      createAgentNode('writer', 'claude-haiku-4-5', 460, 200, 'You are a writer. Synthesize ideas into clear proposals.'),
      createStageNode('Review', 400, 320, false),
    ],
    edges: [
      {
        id: generateId(),
        source: '0',
        target: '1',
        animated: true,
        style: { stroke: 'hsl(142 76% 36%)' },
      },
      {
        id: generateId(),
        source: '0',
        target: '2',
        animated: true,
        style: { stroke: 'hsl(142 76% 36%)' },
      },
      {
        id: generateId(),
        source: '0',
        target: '3',
        animated: true,
        style: { stroke: 'hsl(142 76% 36%)' },
      },
      {
        id: generateId(),
        source: '1',
        target: '4',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
      {
        id: generateId(),
        source: '2',
        target: '4',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
      {
        id: generateId(),
        source: '3',
        target: '4',
        animated: true,
        style: { stroke: 'hsl(221.2 83.2% 53.3%)' },
      },
    ],
  },
];