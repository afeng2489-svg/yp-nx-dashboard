import { useState, useEffect } from 'react';
import { X, GitBranch, Users, ArrowRight } from 'lucide-react';
import {
  useTemplateStore,
  TEMPLATE_CATEGORIES,
  type TemplateSummary,
  type Template,
  type TemplateCategory,
} from '@/stores/templateStore';
import { TemplateCard, TemplateCardSkeleton } from './TemplateCard';
import { BetterEmptyState } from '@/components/ui/BetterEmptyState';

interface TemplateGalleryProps {
  isOpen: boolean;
  onClose: () => void;
  onUseTemplate: (template: Template) => void;
}

export function TemplateGallery({ isOpen, onClose, onUseTemplate }: TemplateGalleryProps) {
  const {
    templates,
    loading,
    error,
    selectedCategory,
    fetchTemplates,
    fetchTemplatesByCategory,
    getTemplate,
  } = useTemplateStore();

  const [previewTemplate, setPreviewTemplate] = useState<Template | null>(null);

  useEffect(() => {
    if (isOpen && templates.length === 0) {
      fetchTemplates();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const handleCategoryChange = (category: TemplateCategory | 'all') => {
    if (category === 'all') {
      fetchTemplates();
    } else {
      fetchTemplatesByCategory(category);
    }
  };

  const handlePreview = async (summary: TemplateSummary) => {
    const template = await getTemplate(summary.id);
    if (template) {
      setPreviewTemplate(template);
    }
  };


  if (!isOpen) return null;

  return (
    <>
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />

        <div className="relative w-full max-w-4xl bg-card rounded-xl shadow-2xl border border-border overflow-hidden max-h-[90vh] flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
            <div>
              <h2 className="text-lg font-semibold">工作流模板</h2>
              <p className="text-sm text-muted-foreground">选择模板快速创建工作流</p>
            </div>
            <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Category Tabs */}
          <div className="flex items-center gap-1 px-6 py-3 border-b border-border bg-muted/30 shrink-0 overflow-x-auto">
            <button
              onClick={() => handleCategoryChange('all')}
              className={`
                px-4 py-2 text-sm font-medium rounded-lg whitespace-nowrap transition-colors
                ${
                  selectedCategory === null
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                }
              `}
            >
              全部
            </button>{' '}
            {TEMPLATE_CATEGORIES.map((cat) => (
              <button
                key={cat.value}
                onClick={() => handleCategoryChange(cat.value)}
                className={`
                  px-4 py-2 text-sm font-medium rounded-lg whitespace-nowrap transition-colors capitalize
                  ${
                    selectedCategory === cat.value
                      ? 'bg-primary text-primary-foreground'
                      : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                  }
                `}
              >
                {cat.label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto p-6">
            {loading && templates.length === 0 ? (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {[1, 2, 3, 4].map((i) => (
                  <TemplateCardSkeleton key={i} />
                ))}
              </div>
            ) : error ? (
              <BetterEmptyState
                icon={GitBranch}
                title="加载模板失败"
                description={error}
                action={{
                  label: '重试',
                  onClick: () => fetchTemplates(),
                }}
              />
            ) : templates.length === 0 ? (
              <BetterEmptyState
                icon={GitBranch}
                title="暂无模板"
                description="请选择其他分类或稍后再试。"
              />
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {templates.map((template) => (
                  <TemplateCard
                    key={template.id}
                    template={template}
                    onLaunch={() => handlePreview(template)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Preview Modal */}
      {previewTemplate && (
        <TemplatePreviewModal
          template={previewTemplate}
          onClose={() => setPreviewTemplate(null)}
          onUse={() => {
            onUseTemplate(previewTemplate);
            onClose();
          }}
        />
      )}
    </>
  );
}

interface TemplatePreviewModalProps {
  template: Template;
  onClose: () => void;
  onUse: () => void;
}

function TemplatePreviewModal({ template, onClose, onUse }: TemplatePreviewModalProps) {
  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-card rounded-xl shadow-2xl border border-border overflow-hidden max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
          <div>
            <h3 className="font-semibold text-lg">{template.name}</h3>
            <p className="text-xs text-muted-foreground capitalize">{template.category}</p>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Description */}
          <p className="text-sm text-muted-foreground">{template.description}</p>

          {/* Stages */}
          <div>
            <h4 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
              <GitBranch className="w-4 h-4" /> 阶段 ({template.stages.length})
            </h4>
            <div className="space-y-3">
              {template.stages.map((stage, idx) => (
                <div key={idx} className="bg-muted/50 rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="font-medium">{stage.name}</span>
                    {stage.parallel && (
                      <span className="px-2 py-0.5 text-xs bg-blue-500/20 text-blue-600 rounded">
                        并行
                      </span>
                    )}
                  </div>
                  {stage.agents.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {stage.agents.map((agentId, i) => (
                        <span
                          key={i}
                          className="px-2 py-0.5 text-xs bg-background rounded border border-border capitalize"
                        >
                          {agentId}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Agents */}
          <div>
            <h4 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
              <Users className="w-4 h-4" /> 智能体 ({template.agents.length})
            </h4>
            <div className="space-y-2">
              {template.agents.map((agent) => (
                <div key={agent.id} className="bg-muted/50 rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-medium capitalize">{agent.role}</span>
                    <span className="px-2 py-0.5 text-xs bg-muted rounded">{agent.model}</span>
                  </div>
                  <p className="text-xs text-muted-foreground line-clamp-2">{agent.prompt}</p>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-border bg-muted/30 shrink-0 flex items-center gap-3">
          <button
            onClick={onClose}
            className="flex-1 py-2 text-sm rounded-md border border-border hover:bg-accent transition-colors"
          >
            取消
          </button>
          <button
            onClick={onUse}
            className="flex-1 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
          >
            使用模板 <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
