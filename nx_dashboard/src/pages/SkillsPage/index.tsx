import { useState } from 'react';
import { Plus, Download, CheckSquare, Search, Lightbulb } from 'lucide-react';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { useSkillsPage } from './hooks';
import { SkillCard } from './SkillCard';
import { Pagination } from '@/components/ui/Pagination';
import { SkillDetailPanel } from './SkillDetailPanel';
import { SkillEditDialog } from './SkillEditDialog';
import { SkillExecuteDialog } from './SkillExecuteDialog';
import { SkillImportDialog } from './SkillImportDialog';
import { INPUT_CLS } from './types';

const PAGE_SIZE = 6;

export function SkillsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const p = useSkillsPage();
  const [page, setPage] = useState(1);
  const totalPages = Math.ceil(p.displaySkills.length / PAGE_SIZE);
  const pagedSkills = p.displaySkills.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  return (
    <>
      <div className="flex h-full">
        <div className="w-80 border-r border-border flex flex-col bg-card">
          <form onSubmit={p.handleSearch} className="p-4 border-b border-border">
            <div className="relative">
              <input
                type="text"
                value={p.searchQuery}
                onChange={(e) => {
                  p.setSearchQuery(e.target.value);
                  setPage(1);
                }}
                placeholder="搜索技能..."
                className={INPUT_CLS + ' pl-10'}
              />
              <Search className="absolute left-3 top-2.5 w-5 h-5 text-muted-foreground" />
            </div>
          </form>

          <div className="p-4 border-b border-border flex gap-2">
            <button
              onClick={p.handleOpenCreateDialog}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 btn-primary rounded-lg"
            >
              <Plus className="w-4 h-4" /> 新建
            </button>
            <button
              onClick={p.handleOpenImportDialog}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition-colors"
            >
              <Download className="w-4 h-4" /> 导入
            </button>
            <button
              onClick={p.toggleMultiSelectMode}
              className={`flex items-center justify-center gap-1 px-3 py-2 rounded-lg border transition-colors ${
                p.multiSelectMode
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background text-muted-foreground border-border hover:bg-accent'
              }`}
              title={p.multiSelectMode ? '退出多选' : '多选'}
            >
              <CheckSquare className="w-4 h-4" />
            </button>
          </div>

          {p.multiSelectMode && (
            <div className="px-4 py-2 border-b border-border bg-accent/40 flex items-center justify-between gap-2">
              <label className="flex items-center gap-2 text-sm text-foreground cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={
                    p.selectableSkills().length > 0 &&
                    p.selectableSkills().every((s) => p.selectedIds.has(s.id))
                  }
                  onChange={p.toggleSelectAll}
                  className="w-4 h-4 accent-primary"
                />
                全选
                <span className="text-xs text-muted-foreground">（已选 {p.selectedIds.size}）</span>
              </label>
              <button
                onClick={p.handleBatchDelete}
                disabled={p.selectedIds.size === 0 || p.batchDeleting}
                className="flex items-center gap-1 px-3 py-1.5 text-sm text-destructive hover:bg-destructive/10 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {p.batchDeleting ? '删除中...' : '删除所选'}
              </button>
            </div>
          )}

          {p.stats && (
            <div className="p-4 border-b border-border bg-accent/50">
              <div className="text-sm text-muted-foreground">
                共 <span className="font-semibold text-primary">{p.stats.total_skills}</span> 个技能
              </div>
            </div>
          )}

          {p.categories.length > 0 && (
            <div className="p-4 border-b border-border">
              <h3 className="text-sm font-medium text-foreground mb-2">类别</h3>
              <div className="flex flex-wrap gap-2">
                {p.categories.map((cat) => (
                  <button
                    key={cat}
                    onClick={() => {
                      p.handleCategoryClick(cat);
                      setPage(1);
                    }}
                    className={`px-2 py-1 text-xs rounded-full transition-colors ${
                      p.selectedCategory === cat
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-accent text-muted-foreground hover:bg-accent/80'
                    }`}
                  >
                    {cat.replace('_', ' ')}
                  </button>
                ))}
              </div>
            </div>
          )}

          <div className="flex-1 overflow-y-auto">
            {p.isLoading ? (
              <div className="p-4 text-center text-muted-foreground">加载中...</div>
            ) : p.error ? (
              <div className="p-4 text-center text-destructive">{p.error}</div>
            ) : p.displaySkills.length === 0 ? (
              <div className="p-4 text-center text-muted-foreground">没有找到技能</div>
            ) : (
              <>
                <div className="divide-y divide-border">
                  {pagedSkills.map((skill) => (
                    <SkillCard
                      key={skill.id}
                      skill={skill}
                      multiSelectMode={p.multiSelectMode}
                      isSelected={p.selectedIds.has(skill.id)}
                      selectedSkillId={p.selectedSkill?.id}
                      onSkillClick={p.handleSkillClick}
                      onToggleSelect={p.toggleSelectOne}
                    />
                  ))}
                </div>
                <div className="p-2 border-t border-border">
                  <Pagination page={page} totalPages={totalPages} onPageChange={setPage} />
                </div>
              </>
            )}
          </div>
        </div>

        <div className="flex-1 bg-card overflow-y-auto">
          {p.currentSkill ? (
            <SkillDetailPanel
              skill={p.currentSkill}
              executing={p.executing}
              onOpenEditDialog={p.handleOpenEditDialog}
              onDelete={p.handleDeleteSkill}
              onToggleEnabled={p.toggleSkillEnabled}
              onOpenExecuteDialog={p.handleOpenExecuteDialog}
            />
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <Lightbulb className="w-16 h-16 mx-auto mb-4 text-muted-foreground/30" />
                <p>选择一个技能查看详情</p>
              </div>
            </div>
          )}
        </div>

        {p.showEditDialog && (
          <SkillEditDialog
            isCreating={p.isCreating}
            editForm={p.editForm}
            saving={p.saving}
            inputCls={INPUT_CLS}
            onFormChange={p.setEditForm}
            onClose={p.handleCloseEditDialog}
            onSave={p.handleSaveSkill}
          />
        )}
        {p.showExecuteDialog && p.currentSkill && (
          <SkillExecuteDialog
            skillName={p.currentSkill.name}
            parameters={p.currentSkill.parameters}
            paramValues={p.paramValues}
            executionResult={p.executionResult}
            executing={p.executing}
            inputCls={INPUT_CLS}
            onParamChange={p.handleParamChange}
            onClose={p.handleCloseExecuteDialog}
            onExecute={p.handleExecuteSkill}
          />
        )}
        {p.showImportDialog && (
          <SkillImportDialog
            importMode={p.importMode}
            importContent={p.importContent}
            importFilename={p.importFilename}
            importing={p.importing}
            importPreview={p.importPreview}
            inputCls={INPUT_CLS}
            onModeChange={p.setImportMode}
            onContentChange={p.handleImportContentChange}
            onFileSelect={p.handleFileSelect}
            onFileDrop={p.handleFileDrop}
            onClose={p.handleCloseImportDialog}
            onImport={p.handleImport}
          />
        )}
      </div>

      <ConfirmModal
        isOpen={p.confirmState.isOpen}
        title={p.confirmState.title}
        message={p.confirmState.message}
        onConfirm={p.confirmState.onConfirm}
        onCancel={p.hideConfirm}
      />
    </>
  );
}

export default SkillsPage;
