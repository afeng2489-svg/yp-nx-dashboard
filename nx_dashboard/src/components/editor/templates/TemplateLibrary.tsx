import { useState } from 'react';
import { WorkflowTemplate, WorkflowNode } from '../types';
import { useEditorStore } from '@/stores/editorStore';
import { X, GitBranch, Users, Eye, ArrowRight } from 'lucide-react';

interface TemplateLibraryProps {
  templates: WorkflowTemplate[];
}

// Template Preview Modal Component
function TemplatePreviewModal({ template, onClose, onLoad }: {
  template: WorkflowTemplate;
  onClose: () => void;
  onLoad: () => void;
}) {
  // Analyze template structure
  const stageNodes = template.nodes.filter(n => n.data.type === 'stage');
  const agentNodes = template.nodes.filter(n => n.data.type === 'agent');

  // Get stage config
  const getStageConfig = (node: WorkflowNode) => {
    if (node.data.type === 'stage' && 'parallel' in node.data.config) {
      return node.data.config;
    }
    return null;
  };

  // Get agent config
  const getAgentConfig = (node: WorkflowNode) => {
    if (node.data.type === 'agent' && 'role' in node.data.config) {
      return node.data.config;
    }
    return null;
  };

  // Find agents for a stage
  const getAgentsForStage = (stageId: string) => {
    const agentConfigs: ReturnType<typeof getAgentConfig>[] = [];
    template.edges.forEach(edge => {
      if (edge.source === stageId) {
        const agentNode = agentNodes.find(n => n.id === edge.target);
        if (agentNode) {
          agentConfigs.push(getAgentConfig(agentNode));
        }
      }
    });
    return agentConfigs.filter(Boolean);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md bg-card rounded-xl shadow-2xl border border-border overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <div>
            <h3 className="font-semibold">{template.name}</h3>
            <p className="text-xs text-muted-foreground capitalize">{template.category}</p>
          </div>
          <button onClick={onClose} className="p-1 rounded hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-6 space-y-6 max-h-96 overflow-y-auto">
          <p className="text-sm text-muted-foreground">{template.description}</p>

          {/* Stages */}
          <div>
            <h4 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
              <GitBranch className="w-4 h-4" /> Stages ({stageNodes.length})
            </h4>
            <div className="space-y-3">
              {stageNodes.map((stage) => {
                const config = getStageConfig(stage);
                const stageAgents = getAgentsForStage(stage.id);
                return (
                  <div key={stage.id} className="bg-muted/50 rounded-lg p-3 border border-border">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="font-medium">{stage.data.label}</span>
                      {config && config.parallel && (
                        <span className="px-2 py-0.5 text-xs bg-blue-500/20 text-blue-600 rounded">
                          Parallel
                        </span>
                      )}
                    </div>
                    {stageAgents.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {stageAgents.map((agent, i) => (
                          agent && (
                            <span key={i} className="px-2 py-0.5 text-xs bg-background rounded border border-border capitalize">
                              {agent.role}
                            </span>
                          )
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* All Agents */}
          <div>
            <h4 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
              <Users className="w-4 h-4" /> All Agents ({agentNodes.length})
            </h4>
            <div className="space-y-2">
              {agentNodes.map((agent) => {
                const config = getAgentConfig(agent);
                if (!config) return null;
                return (
                  <div key={agent.id} className="bg-muted/50 rounded-lg p-3 border border-border">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-medium capitalize">{config.role}</span>
                      <span className="px-2 py-0.5 text-xs bg-muted rounded">{config.model}</span>
                    </div>
                    <p className="text-xs text-muted-foreground line-clamp-2">{config.prompt}</p>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        <div className="px-6 py-4 border-t border-border bg-muted/30">
          <button
            onClick={onLoad}
            className="w-full py-2 bg-primary text-primary-foreground rounded-md hover:opacity-90 flex items-center justify-center gap-2"
          >
            Use Template <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

export function TemplateLibrary({ templates }: TemplateLibraryProps) {
  const { loadTemplate } = useEditorStore();
  const [isOpen, setIsOpen] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [previewTemplate, setPreviewTemplate] = useState<WorkflowTemplate | null>(null);

  const categories = ['all', ...new Set(templates.map((t) => t.category))];

  const filteredTemplates =
    selectedCategory === 'all'
      ? templates
      : templates.filter((t) => t.category === selectedCategory);

  const handleLoadTemplate = (template: WorkflowTemplate) => {
    loadTemplate({ nodes: template.nodes, edges: template.edges });
    setIsOpen(false);
    setPreviewTemplate(null);
  };

  return (
    <>
      <button
        onClick={() => setIsOpen(true)}
        className="
          flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-lg
          bg-primary text-primary-foreground
          hover:bg-primary/90 transition-colors
        "
      >
        <svg
          className="w-4 h-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"
          />
        </svg>
        Templates
      </button>

      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={() => setIsOpen(false)}
          />

          <div className="relative w-full max-w-2xl bg-card rounded-xl shadow-2xl border border-border overflow-hidden">
            <div className="flex items-center justify-between px-6 py-4 border-b border-border">
              <h2 className="text-lg font-semibold">Workflow Templates</h2>
              <button
                onClick={() => setIsOpen(false)}
                className="p-1 rounded hover:bg-accent transition-colors"
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>

            <div className="flex border-b border-border">
              {categories.map((cat) => (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={`
                    px-4 py-2 text-sm font-medium capitalize transition-colors
                    ${selectedCategory === cat
                      ? 'text-primary border-b-2 border-primary'
                      : 'text-muted-foreground hover:text-foreground'
                    }
                  `}
                >
                  {cat}
                </button>
              ))}
            </div>

            <div className="p-4 grid grid-cols-2 gap-4 max-h-96 overflow-y-auto">
              {filteredTemplates.map((template) => (
                <div
                  key={template.id}
                  className="
                    p-4 rounded-lg border border-border text-left
                    hover:border-primary hover:bg-accent/50 transition-all
                  "
                >
                  <div className="flex items-center gap-2 mb-2">
                    <TemplateIcon category={template.category} />
                    <h3 className="font-medium">{template.name}</h3>
                  </div>
                  <p className="text-sm text-muted-foreground line-clamp-2">
                    {template.description}
                  </p>
                  <div className="mt-2 flex items-center gap-2">
                    <span className="px-2 py-0.5 text-xs bg-muted rounded capitalize">
                      {template.category}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {template.nodes.length} nodes
                    </span>
                  </div>
                  <div className="mt-3 flex items-center gap-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setPreviewTemplate(template);
                      }}
                      className="
                        px-2 py-1 text-xs rounded border border-border
                        hover:bg-accent hover:border-accent transition-colors
                        flex items-center gap-1
                      "
                    >
                      <Eye className="w-3 h-3" /> Preview
                    </button>
                    <button
                      onClick={() => handleLoadTemplate(template)}
                      className="
                        px-2 py-1 text-xs rounded bg-primary text-primary-foreground
                        hover:opacity-90 transition-opacity flex items-center gap-1
                      "
                    >
                      Use <ArrowRight className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {previewTemplate && (
        <TemplatePreviewModal
          template={previewTemplate}
          onClose={() => setPreviewTemplate(null)}
          onLoad={() => handleLoadTemplate(previewTemplate)}
        />
      )}
    </>
  );
}

function TemplateIcon({ category }: { category: string }) {
  switch (category) {
    case 'basic':
      return <span className="text-lg">📋</span>;
    case 'collaboration':
      return <span className="text-lg">👥</span>;
    case 'testing':
      return <span className="text-lg">🧪</span>;
    case 'brainstorm':
      return <span className="text-lg">💡</span>;
    default:
      return <span className="text-lg">📄</span>;
  }
}