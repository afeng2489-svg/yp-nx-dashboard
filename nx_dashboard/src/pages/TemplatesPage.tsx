import { useState, useEffect } from 'react';
import { TemplateCard, TemplateCardSkeleton } from '@/components/workflow/TemplateCard';
import { useTemplateStore, TEMPLATE_CATEGORIES, type TemplateSummary, type TemplateCategory } from '@/stores/templateStore';

export function TemplatesPage() {
  const {
    templates,
    loading,
    error,
    selectedCategory,
    fetchTemplates,
    fetchTemplatesByCategory,
    getTemplate,
    instantiateTemplate,
  } = useTemplateStore();

  const [previewTemplate, setPreviewTemplate] = useState<any>(null);

  useEffect(() => {
    fetchTemplates();
  }, []);

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

  const handleUseTemplate = async (summary: TemplateSummary) => {
    await instantiateTemplate(summary.id);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">工作流模板</h1>
          <p className="text-sm text-muted-foreground mt-1">
            选择模板快速创建工作流
          </p>
        </div>
      </div>

      {/* Category tabs */}
      <div className="px-4 border-b flex gap-2 overflow-x-auto">
        <button
          onClick={() => handleCategoryChange('all')}
          className={`px-3 py-2 text-sm rounded-lg transition-colors ${
            selectedCategory === null
              ? 'bg-primary text-primary-foreground'
              : 'hover:bg-accent'
          }`}
        >
          全部
        </button>
        {TEMPLATE_CATEGORIES.map((cat) => (
          <button
            key={cat.value}
            onClick={() => handleCategoryChange(cat.value)}
            className={`px-3 py-2 text-sm rounded-lg transition-colors ${
              selectedCategory === cat.value
                ? 'bg-primary text-primary-foreground'
                : 'hover:bg-accent'
            }`}
          >
            {cat.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4">
        {loading && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {[1, 2, 3].map((i) => (
              <TemplateCardSkeleton key={i} />
            ))}
          </div>
        )}
        {error && (
          <div className="flex items-center justify-center h-64 text-destructive">
            {error}
          </div>
        )}
        {!loading && !error && templates.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
            <p>暂无模板</p>
          </div>
        )}
        {!loading && !error && templates.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {templates.map((template) => (
              <TemplateCard
                key={template.id}
                template={template}
                onPreview={() => handlePreview(template)}
                onUse={() => handleUseTemplate(template)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
