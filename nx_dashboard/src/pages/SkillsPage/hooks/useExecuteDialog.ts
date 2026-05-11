import { useState, useCallback } from 'react';
import { useSkillStore, type SkillDetail } from '@/stores/skillStore';
import { showSuccess, showError } from '@/lib/toast';

export function useExecuteDialog(currentSkill: SkillDetail | null) {
  const { executing, executeSkill } = useSkillStore();

  const [showExecuteDialog, setShowExecuteDialog] = useState(false);
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [executionResult, setExecutionResult] = useState<string | null>(null);

  const handleOpenExecuteDialog = useCallback(() => {
    if (!currentSkill) return;
    const initialValues: Record<string, string> = {};
    currentSkill.parameters.forEach((param) => {
      initialValues[param.name] =
        param.default !== null && param.default !== undefined ? String(param.default) : '';
    });
    setParamValues(initialValues);
    setExecutionResult(null);
    setShowExecuteDialog(true);
  }, [currentSkill]);

  const handleCloseExecuteDialog = useCallback(() => {
    setShowExecuteDialog(false);
    setParamValues({});
    setExecutionResult(null);
  }, []);

  const handleParamChange = useCallback((name: string, value: string) => {
    setParamValues((prev) => ({ ...prev, [name]: value }));
  }, []);

  const handleExecuteSkill = useCallback(async () => {
    if (!currentSkill) return;

    const params: Record<string, unknown> = {};
    currentSkill.parameters.forEach((param) => {
      const value = paramValues[param.name];
      if (value !== undefined && value !== '') {
        try {
          params[param.name] = JSON.parse(value);
        } catch {
          params[param.name] = value;
        }
      }
    });

    try {
      const result = await executeSkill({ skill_id: currentSkill.id, params });

      if (result.success) {
        showSuccess('技能执行成功', `耗时: ${result.duration_ms}ms`);
        setExecutionResult(
          result.output !== null ? JSON.stringify(result.output, null, 2) : '执行完成，无输出',
        );
      } else {
        showError('技能执行失败', result.error || '未知错误');
        setExecutionResult(`执行失败: ${result.error}`);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : '执行出错';
      showError('技能执行失败', message);
      setExecutionResult(`执行失败: ${message}`);
    }
  }, [currentSkill, paramValues, executeSkill]);

  return {
    showExecuteDialog,
    executing,
    paramValues,
    executionResult,
    handleOpenExecuteDialog,
    handleCloseExecuteDialog,
    handleExecuteSkill,
    handleParamChange,
  };
}
