import { X, GitBranch, Users, AlertCircle, ChevronRight, Pause, Terminal, Info } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { Workflow, Stage, Agent } from '@/stores/workflowStore';

interface WorkflowTutorialModalProps {
  workflow: Workflow;
  onClose: () => void;
}

function ModelBadge({ model }: { model: string }) {
  const label = model.includes('opus') ? 'Opus' : model.includes('sonnet') ? 'Sonnet' : model.includes('haiku') ? 'Haiku' : model;
  const cls = model.includes('opus')
    ? 'bg-amber-500/10 text-amber-600 border-amber-500/20'
    : model.includes('sonnet')
    ? 'bg-indigo-500/10 text-indigo-600 border-indigo-500/20'
    : 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20';
  return (
    <span className={cn('px-1.5 py-0.5 rounded text-[10px] font-medium border', cls)}>
      {label}
    </span>
  );
}

function StageNode({ stage, index, total, agents }: { stage: Stage; index: number; total: number; agents: Agent[] }) {
  const isUserInput = stage.stage_type === 'user_input';
  const stageAgents = (stage.agents ?? [])
    .map(id => agents.find(a => a.id === id))
    .filter(Boolean) as Agent[];

  return (
    <div className="flex flex-col items-center">
      <div className={cn(
        'w-full rounded-xl border p-3',
        isUserInput
          ? 'bg-amber-500/5 border-amber-500/20'
          : 'bg-gradient-to-r from-indigo-500/5 to-purple-500/5 border-indigo-500/15'
      )}>
        <div className="flex items-center gap-2 mb-1">
          <span className={cn(
            'w-5 h-5 rounded-full text-[10px] font-bold flex items-center justify-center flex-shrink-0',
            isUserInput ? 'bg-amber-500 text-white' : 'bg-indigo-500 text-white'
          )}>
            {isUserInput ? <Pause className="w-2.5 h-2.5" /> : index + 1}
          </span>
          <span className="text-sm font-medium">{stage.name}</span>
          {isUserInput && (
            <span className="ml-auto px-1.5 py-0.5 rounded text-[10px] bg-amber-500/15 text-amber-600 border border-amber-500/20">
              等待输入
            </span>
          )}
        </div>
        {isUserInput && stage.question && (
          <p className="text-xs text-muted-foreground ml-7 mb-1">{stage.question}</p>
        )}
        {stageAgents.length > 0 && (
          <div className="ml-7 flex flex-wrap gap-1">
            {stageAgents.map(agent => (
              <span key={agent.id} className="inline-flex items-center gap-1 text-[10px] text-muted-foreground">
                <span className="w-1 h-1 rounded-full bg-indigo-400 flex-shrink-0" />
                {agent.role}
                <ModelBadge model={agent.model} />
              </span>
            ))}
          </div>
        )}
      </div>
      {index < total - 1 && (
        <div className="flex flex-col items-center py-0.5">
          <ChevronRight className="w-3 h-3 text-muted-foreground rotate-90" />
        </div>
      )}
    </div>
  );
}

export function WorkflowTutorialModal({ workflow, onClose }: WorkflowTutorialModalProps) {
  const requiredInputs = workflow.triggers
    ?.flatMap(t => Object.entries(t.inputs ?? {}))
    .filter(([, v]) => v.required) ?? [];

  const optionalInputs = workflow.triggers
    ?.flatMap(t => Object.entries(t.inputs ?? {}))
    .filter(([, v]) => !v.required) ?? [];

  const hasUserInputStage = workflow.stages.some(s => s.stage_type === 'user_input');

  const steps: string[] = [];
  if (requiredInputs.length > 0) {
    steps.push(`点击"执行"按钮，填写必填参数：${requiredInputs.map(([k]) => k).join('、')}`);
  } else {
    steps.push('直接点击绿色"执行"按钮，无需填写参数');
  }
  steps.push('系统跳转到"执行"页面，可实时查看各阶段日志');
  if (hasUserInputStage) {
    steps.push('工作流会在"等待输入"阶段暂停，页面弹出选项，按提示选择后继续执行');
  }
  steps.push('执行完成后查看最终报告或输出结果');

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="bg-card border border-border rounded-2xl w-full max-w-2xl max-h-[85vh] flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-start justify-between p-5 border-b border-border flex-shrink-0">
          <div className="flex-1 min-w-0 pr-3">
            <div className="flex items-center gap-2 mb-1">
              <h2 className="text-lg font-bold truncate">{workflow.name}</h2>
              <span className="px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-600 text-xs font-medium flex-shrink-0">
                v{workflow.version}
              </span>
            </div>
            {workflow.description && (
              <p className="text-sm text-muted-foreground">{workflow.description}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors flex-shrink-0"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="overflow-y-auto flex-1 p-5 space-y-5">
          {/* Required Inputs */}
          {(requiredInputs.length > 0 || optionalInputs.length > 0) && (
            <section>
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                <Terminal className="w-4 h-4 text-indigo-500" />
                执行参数
              </h3>
              <div className="space-y-2">
                {requiredInputs.map(([name, input]) => (
                  <div key={name} className="flex items-start gap-3 p-3 rounded-xl bg-red-500/5 border border-red-500/15">
                    <AlertCircle className="w-4 h-4 text-red-500 flex-shrink-0 mt-0.5" />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 mb-0.5">
                        <code className="text-sm font-mono font-semibold text-red-600">{name}</code>
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-red-500/10 text-red-500 border border-red-500/20">必填</span>
                        <span className="text-[10px] text-muted-foreground">{input.type}</span>
                      </div>
                      <p className="text-xs text-muted-foreground">{input.description}</p>
                    </div>
                  </div>
                ))}
                {optionalInputs.map(([name, input]) => (
                  <div key={name} className="flex items-start gap-3 p-3 rounded-xl bg-accent/50 border border-border">
                    <Info className="w-4 h-4 text-muted-foreground flex-shrink-0 mt-0.5" />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 mb-0.5">
                        <code className="text-sm font-mono font-semibold">{name}</code>
                        <span className="text-[10px] text-muted-foreground">{input.type}（选填）</span>
                      </div>
                      {input.description && <p className="text-xs text-muted-foreground">{input.description}</p>}
                    </div>
                  </div>
                ))}
              </div>
            </section>
          )}

          {/* Execution Flow */}
          {workflow.stages.length > 0 && (
            <section>
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                <GitBranch className="w-4 h-4 text-indigo-500" />
                执行流程（{workflow.stages.length} 个阶段）
              </h3>
              <div className="space-y-0">
                {workflow.stages.map((stage, i) => (
                  <StageNode
                    key={stage.name}
                    stage={stage}
                    index={i}
                    total={workflow.stages.length}
                    agents={workflow.agents}
                  />
                ))}
              </div>
            </section>
          )}

          {/* Agents Overview */}
          {workflow.agents.length > 0 && (
            <section>
              <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                <Users className="w-4 h-4 text-purple-500" />
                参与智能体（{workflow.agents.length} 个）
              </h3>
              <div className="grid gap-2">
                {workflow.agents.map(agent => (
                  <div key={agent.id} className="flex items-start gap-3 p-3 rounded-xl bg-gradient-to-r from-purple-500/5 to-pink-500/5 border border-purple-500/10">
                    <div className="w-6 h-6 rounded-lg bg-purple-500/15 flex items-center justify-center flex-shrink-0 mt-0.5">
                      <span className="text-[10px] font-bold text-purple-600">AI</span>
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 mb-0.5">
                        <span className="text-sm font-medium capitalize">{agent.role}</span>
                        <ModelBadge model={agent.model} />
                      </div>
                      <p className="text-xs text-muted-foreground line-clamp-2">
                        {agent.prompt.trim().split('\n').find(l => l.trim().length > 0) ?? ''}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          )}

          {/* Usage Steps */}
          <section>
            <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Info className="w-4 h-4 text-emerald-500" />
              使用步骤
            </h3>
            <ol className="space-y-2">
              {steps.map((step, i) => (
                <li key={i} className="flex items-start gap-3 text-sm">
                  <span className="w-5 h-5 rounded-full bg-emerald-500 text-white text-[10px] font-bold flex items-center justify-center flex-shrink-0 mt-0.5">
                    {i + 1}
                  </span>
                  <span className="text-muted-foreground">{step}</span>
                </li>
              ))}
            </ol>
          </section>

          {hasUserInputStage && (
            <div className="flex items-start gap-3 p-3 rounded-xl bg-amber-500/5 border border-amber-500/20">
              <Pause className="w-4 h-4 text-amber-500 flex-shrink-0 mt-0.5" />
              <p className="text-xs text-amber-700 dark:text-amber-400">
                此工作流包含<strong>人工干预节点</strong>，执行期间会暂停并弹出选项供你选择，请保持页面在线并留意执行状态。
              </p>
            </div>
          )}
        </div>

        <div className="p-4 border-t border-border flex-shrink-0">
          <button
            onClick={onClose}
            className="w-full py-2.5 rounded-xl bg-gradient-to-r from-indigo-500 to-purple-500 text-white text-sm font-medium hover:shadow-lg hover:shadow-indigo-500/25 transition-all"
          >
            明白了
          </button>
        </div>
      </div>
    </div>
  );
}
