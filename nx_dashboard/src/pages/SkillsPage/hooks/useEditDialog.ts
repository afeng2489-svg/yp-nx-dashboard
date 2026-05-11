import { useState, useCallback } from 'react';
import {
  useSkillStore,
  type SkillDetail,
  type CreateSkillRequest,
  type UpdateSkillRequest,
} from '@/stores/skillStore';
import { showSuccess, showError } from '@/lib/toast';

const EMPTY_FORM: CreateSkillRequest = {
  id: '',
  name: '',
  description: '',
  category: 'development',
  tags: [],
  parameters: [],
};

export function useEditDialog(
  currentSkill: SkillDetail | null,
  refetchSkills: () => void,
  fetchStats: () => void,
) {
  const { saving, createSkill, updateSkill } = useSkillStore();

  const [showEditDialog, setShowEditDialog] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [editForm, setEditForm] = useState<CreateSkillRequest>({ ...EMPTY_FORM });

  const handleOpenCreateDialog = useCallback(() => {
    setIsCreating(true);
    setEditForm({ ...EMPTY_FORM });
    setShowEditDialog(true);
  }, []);

  const handleOpenEditDialog = useCallback(() => {
    if (!currentSkill) return;
    setIsCreating(false);
    setEditForm({
      id: currentSkill.id,
      name: currentSkill.name,
      description: currentSkill.description,
      category: currentSkill.category,
      tags: currentSkill.tags,
      parameters: currentSkill.parameters,
      code: currentSkill.code || '',
    });
    setShowEditDialog(true);
  }, [currentSkill]);

  const handleCloseEditDialog = useCallback(() => {
    setShowEditDialog(false);
    setEditForm({ ...EMPTY_FORM, code: '' });
  }, []);

  const handleSaveSkill = useCallback(async () => {
    if (!editForm.id.trim() || !editForm.name.trim()) {
      showError('验证失败', 'ID 和名称不能为空');
      return;
    }

    let result;
    if (isCreating) {
      result = await createSkill({ ...editForm, code: editForm.code });
      if (result) {
        showSuccess('创建成功', `技能 "${result.name}" 已创建`);
      } else {
        showError('创建失败', useSkillStore.getState().error || '未知错误');
        return;
      }
    } else {
      const updateData: UpdateSkillRequest = {
        name: editForm.name,
        description: editForm.description,
        category: editForm.category,
        tags: editForm.tags,
        code: editForm.code,
      };
      result = await updateSkill(editForm.id, updateData);
      if (result) {
        showSuccess('更新成功', `技能 "${result.name}" 已更新`);
      } else {
        showError('更新失败', useSkillStore.getState().error || '未知错误');
        return;
      }
    }

    handleCloseEditDialog();
    refetchSkills();
    fetchStats();
  }, [
    editForm,
    isCreating,
    createSkill,
    updateSkill,
    handleCloseEditDialog,
    refetchSkills,
    fetchStats,
  ]);

  return {
    showEditDialog,
    isCreating,
    editForm,
    saving,
    setEditForm,
    handleOpenCreateDialog,
    handleOpenEditDialog,
    handleCloseEditDialog,
    handleSaveSkill,
  };
}
