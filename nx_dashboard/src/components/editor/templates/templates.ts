import { Node } from '@xyflow/react';
import { WorkflowTemplate, WorkflowNodeData } from '../types';

const generateId = (): string =>
  `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;

const createAgentNode = (
  role: string,
  model: string,
  x: number,
  y: number,
  prompt?: string,
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
  parallel: boolean = false,
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
    name: '单智能体任务',
    description: '单个智能体执行任务',
    category: 'planning',
    nodes: [createAgentNode('planner', 'claude-opus-4-6', 400, 200)],
    edges: [],
  },
  {
    id: 'multi-cli-plan',
    name: '多智能体并行',
    description: '多个智能体并行协作执行',
    category: 'development',
    nodes: [
      createStageNode('并行任务', 400, 100, true),
      createAgentNode(
        'developer',
        'claude-sonnet-4-6',
        250,
        250,
        '你是一位开发工程师，负责编写代码。',
      ),
      createAgentNode(
        'reviewer',
        'claude-sonnet-4-6',
        400,
        250,
        '你是一位代码审查员，负责审查代码质量。',
      ),
      createAgentNode(
        'tester',
        'claude-sonnet-4-6',
        550,
        250,
        '你是一位测试工程师，负责编写和执行测试。',
      ),
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
    name: 'TDD 测试驱动开发',
    description: '测试驱动开发循环：先写测试，再实现，最后审查',
    category: 'testing',
    nodes: [
      createStageNode('编写测试', 300, 100, false),
      createAgentNode('tester', 'claude-haiku-4-5', 150, 220, '为下一个功能编写失败的测试用例。'),
      createAgentNode('developer', 'claude-opus-4-6', 300, 220, '实现功能代码使测试通过。'),
      createAgentNode('reviewer', 'claude-sonnet-4-6', 450, 220, '审查代码实现质量。'),
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
    name: '多角色头脑风暴',
    description: '规划者、研究者、写作者协作头脑风暴',
    category: 'planning',
    nodes: [
      createStageNode('头脑风暴', 400, 80, true),
      createAgentNode(
        'planner',
        'claude-opus-4-6',
        200,
        200,
        '你是规划者，将问题拆解为可行动的步骤。',
      ),
      createAgentNode(
        'researcher',
        'claude-sonnet-4-6',
        330,
        200,
        '你是研究者，收集相关信息和背景知识。',
      ),
      createAgentNode(
        'writer',
        'claude-haiku-4-5',
        460,
        200,
        '你是写作者，将想法综合成清晰的方案。',
      ),
      createStageNode('审查', 400, 320, false),
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
