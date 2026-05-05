import { useMemo } from 'react';
import {
  useEditorStore,
  AgentConfig,
  StageConfig,
  ConditionConfig,
  LoopConfig,
} from '@/stores/editorStore';
import { AGENT_ROLES, CLI_PROVIDERS, MODEL_OPTIONS, NODE_COLORS, NODE_ICONS } from './types';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';

export function PropertyPanel() {
  const { nodes, selectedNodeId, updateNodeData, deleteNode } = useEditorStore();

  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId),
    [nodes, selectedNodeId],
  );

  if (!selectedNode) {
    return (
      <div className="w-80 bg-card border border-border rounded-lg shadow-md p-6">
        <div className="text-center text-muted-foreground">
          <div className="mb-3">
            <svg
              className="mx-auto h-10 w-10 text-muted-foreground/50"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122"
              />
            </svg>
          </div>
          <p className="text-sm">选择节点以编辑属性</p>
        </div>
      </div>
    );
  }

  const { data } = selectedNode;
  const color = NODE_COLORS[data.type];
  const icon = NODE_ICONS[data.type];

  const handleLabelChange = (label: string) => {
    updateNodeData(selectedNode.id, { label });
  };

  const handleConfigChange = (
    config: Partial<AgentConfig | StageConfig | ConditionConfig | LoopConfig>,
  ) => {
    updateNodeData(selectedNode.id, {
      config: { ...data.config, ...config },
    });
  };

  const handleDelete = () => {
    deleteNode(selectedNode.id);
  };

  return (
    <div className="w-80 bg-card border border-border rounded-lg shadow-md overflow-hidden">
      <div
        className="px-4 py-3 border-b border-border"
        style={{ borderLeftColor: color, borderLeftWidth: 4 }}
      >
        <div className="flex items-center gap-2">
          <span className="text-lg">{icon}</span>
          <div>
            <h3 className="font-semibold text-sm capitalize">{data.type}</h3>
            <p className="text-xs text-muted-foreground">配置节点</p>
          </div>
        </div>
      </div>

      <div className="p-4 space-y-4">
        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1.5">标签</label>
          <input
            type="text"
            value={data.label}
            onChange={(e) => handleLabelChange(e.target.value)}
            className="
              w-full px-3 py-2 text-sm rounded-md
              border border-input bg-background
              focus:outline-none focus:ring-2 focus:ring-primary
            "
          />
        </div>

        {data.type === 'agent' && (
          <AgentConfigPanel config={data.config as AgentConfig} onChange={handleConfigChange} />
        )}

        {data.type === 'stage' && (
          <StageConfigPanel config={data.config as StageConfig} onChange={handleConfigChange} />
        )}

        {data.type === 'condition' && (
          <ConditionConfigPanel
            config={data.config as ConditionConfig}
            onChange={handleConfigChange}
          />
        )}

        {data.type === 'loop' && (
          <LoopConfigPanel config={data.config as LoopConfig} onChange={handleConfigChange} />
        )}

        <button
          onClick={handleDelete}
          className="
            w-full mt-4 px-4 py-2 text-sm font-medium rounded-md
            bg-destructive text-destructive-foreground
            hover:bg-destructive/90 transition-colors
          "
        >
          删除节点
        </button>
      </div>
    </div>
  );
}

function AgentConfigPanel({
  config,
  onChange,
}: {
  config: AgentConfig;
  onChange: (config: Partial<AgentConfig>) => void;
}) {
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">角色</label>
        <Select value={config.role} onValueChange={(v) => onChange({ role: v })}>
          <SelectTrigger className="h-8 text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AGENT_ROLES.map((role) => (
              <SelectItem key={role.value} value={role.value}>
                {role.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">模型</label>
        <Select value={config.model} onValueChange={(v) => onChange({ model: v })}>
          <SelectTrigger className="h-8 text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {MODEL_OPTIONS.map((model) => (
              <SelectItem key={model.value} value={model.value}>
                {model.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">CLI 提供商</label>
        <Select
          value={config.cliProvider}
          onValueChange={(v) => onChange({ cliProvider: v as AgentConfig['cliProvider'] })}
        >
          <SelectTrigger className="h-8 text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CLI_PROVIDERS.map((provider) => (
              <SelectItem key={provider.value} value={provider.value}>
                {provider.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">系统提示词</label>
        <textarea
          value={config.prompt}
          onChange={(e) => onChange({ prompt: e.target.value })}
          rows={4}
          className="
            w-full px-3 py-2 text-sm rounded-md resize-none
            border border-input bg-background
            focus:outline-none focus:ring-2 focus:ring-primary
          "
          placeholder="你是一位有用的助手..."
        />
      </div>
    </div>
  );
}

function StageConfigPanel({
  config,
  onChange,
}: {
  config: StageConfig;
  onChange: (config: Partial<StageConfig>) => void;
}) {
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">阶段名称</label>
        <input
          type="text"
          value={config.name}
          onChange={(e) => onChange({ name: e.target.value })}
          className="
            w-full px-3 py-2 text-sm rounded-md
            border border-input bg-background
            focus:outline-none focus:ring-2 focus:ring-primary
          "
        />
      </div>

      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          id="parallel"
          checked={config.parallel}
          onChange={(e) => onChange({ parallel: e.target.checked })}
          className="w-4 h-4 rounded border-input text-primary focus:ring-primary"
        />
        <label htmlFor="parallel" className="text-sm">
          并行运行智能体
        </label>
      </div>
    </div>
  );
}

function ConditionConfigPanel({
  config,
  onChange,
}: {
  config: ConditionConfig;
  onChange: (config: Partial<ConditionConfig>) => void;
}) {
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">条件表达式</label>
        <input
          type="text"
          value={config.expression}
          onChange={(e) => onChange({ expression: e.target.value })}
          className="
            w-full px-3 py-2 text-sm rounded-md
            border border-input bg-background
            focus:outline-none focus:ring-2 focus:ring-primary
          "
          placeholder="e.g., result.status === 'success'"
        />
        <p className="mt-1 text-xs text-muted-foreground">返回 true/false 的 JavaScript 表达式</p>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1.5">
            为真时标签
          </label>
          <input
            type="text"
            value={config.trueLabel}
            onChange={(e) => onChange({ trueLabel: e.target.value })}
            className="
              w-full px-3 py-2 text-sm rounded-md
              border border-input bg-background
              focus:outline-none focus:ring-2 focus:ring-primary
            "
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1.5">
            为假时标签
          </label>
          <input
            type="text"
            value={config.falseLabel}
            onChange={(e) => onChange({ falseLabel: e.target.value })}
            className="
              w-full px-3 py-2 text-sm rounded-md
              border border-input bg-background
              focus:outline-none focus:ring-2 focus:ring-primary
            "
          />
        </div>
      </div>
    </div>
  );
}

function LoopConfigPanel({
  config,
  onChange,
}: {
  config: LoopConfig;
  onChange: (config: Partial<LoopConfig>) => void;
}) {
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-xs font-medium text-muted-foreground mb-1.5">
          最大迭代次数
        </label>
        <input
          type="number"
          min={1}
          max={100}
          value={config.maxIterations}
          onChange={(e) => onChange({ maxIterations: parseInt(e.target.value) || 1 })}
          className="
            w-full px-3 py-2 text-sm rounded-md
            border border-input bg-background
            focus:outline-none focus:ring-2 focus:ring-primary
          "
        />
      </div>

      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          id="continueOnError"
          checked={config.continueOnError}
          onChange={(e) => onChange({ continueOnError: e.target.checked })}
          className="w-4 h-4 rounded border-input text-primary focus:ring-primary"
        />
        <label htmlFor="continueOnError" className="text-sm">
          出错时继续
        </label>
      </div>
    </div>
  );
}
