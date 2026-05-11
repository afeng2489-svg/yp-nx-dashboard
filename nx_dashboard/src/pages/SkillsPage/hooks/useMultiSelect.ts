import { useState, useCallback } from 'react';
import { useSkillStore, type SkillDetail, type SkillSummary } from '@/stores/skillStore';
import { showSuccess, showError } from '@/lib/toast';
import { useConfirmModal } from '@/lib/ConfirmModal';

export function useMultiSelect(
  displaySkills: SkillSummary[],
  currentSkill: SkillDetail | null,
  setSelectedSkill: (skill: SkillSummary | null) => void,
  refetchSkills: () => void,
  fetchStats: () => void,
) {
  const { deleteSkill, clearCurrentSkill } = useSkillStore();
  const { confirmState, showConfirm, hideConfirm } = useConfirmModal();

  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [batchDeleting, setBatchDeleting] = useState(false);

  const toggleMultiSelectMode = useCallback(() => {
    setMultiSelectMode((prev) => {
      if (prev) setSelectedIds(new Set());
      return !prev;
    });
  }, []);

  const toggleSelectOne = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const selectableSkills = useCallback(
    (): SkillSummary[] => displaySkills.filter((s) => !s.is_preset),
    [displaySkills],
  );

  const toggleSelectAll = useCallback(() => {
    const all = selectableSkills();
    const allSelected = all.length > 0 && all.every((s) => selectedIds.has(s.id));
    setSelectedIds(allSelected ? new Set() : new Set(all.map((s) => s.id)));
  }, [selectableSkills, selectedIds]);

  const handleDeleteSkill = useCallback(() => {
    if (!currentSkill) return;
    showConfirm(
      '删除技能',
      `确定要删除技能「${currentSkill.name}」吗？此操作不可撤销。`,
      async () => {
        const success = await deleteSkill(currentSkill.id);
        if (success) {
          showSuccess('删除成功', `技能 "${currentSkill.name}" 已删除`);
          clearCurrentSkill();
          setSelectedSkill(null);
          refetchSkills();
          fetchStats();
        } else {
          showError('删除失败', '请稍后重试');
        }
      },
    );
  }, [
    currentSkill,
    showConfirm,
    deleteSkill,
    clearCurrentSkill,
    setSelectedSkill,
    refetchSkills,
    fetchStats,
  ]);

  const handleBatchDelete = useCallback(() => {
    if (selectedIds.size === 0) return;
    showConfirm(
      '批量删除技能',
      `确定要删除选中的 ${selectedIds.size} 个技能吗？此操作不可撤销。`,
      async () => {
        setBatchDeleting(true);
        const ids = Array.from(selectedIds);
        const results = await Promise.all(ids.map((id) => deleteSkill(id)));
        setBatchDeleting(false);

        const successCount = results.filter(Boolean).length;
        const failCount = results.length - successCount;

        if (successCount > 0) {
          showSuccess('批量删除完成', `成功删除 ${successCount} 个技能`);
        }
        if (failCount > 0) {
          showError('部分删除失败', `${failCount} 个技能删除失败`);
        }

        if (currentSkill && selectedIds.has(currentSkill.id)) {
          clearCurrentSkill();
          setSelectedSkill(null);
        }
        setSelectedIds(new Set());
        setMultiSelectMode(false);
        refetchSkills();
        fetchStats();
      },
    );
  }, [
    selectedIds,
    showConfirm,
    deleteSkill,
    currentSkill,
    clearCurrentSkill,
    setSelectedSkill,
    refetchSkills,
    fetchStats,
  ]);

  return {
    multiSelectMode,
    selectedIds,
    batchDeleting,
    toggleMultiSelectMode,
    toggleSelectOne,
    selectableSkills,
    toggleSelectAll,
    handleDeleteSkill,
    handleBatchDelete,
    confirmState,
    hideConfirm,
  };
}
