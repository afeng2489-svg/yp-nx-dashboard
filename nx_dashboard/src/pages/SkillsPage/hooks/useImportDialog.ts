import { useState, useCallback } from 'react';
import { useSkillStore } from '@/stores/skillStore';
import { showSuccess, showError } from '@/lib/toast';
import type { ImportPreview, ImportMode } from '../types';

function parseFrontmatter(content: string): ImportPreview | null {
  const lines = content.split('\n');
  let inFrontmatter = false;
  let name = '';
  let description = '';
  let category = 'general';
  let tags: string[] = ['agent'];

  for (const line of lines) {
    if (line.trim() === '---') {
      if (!inFrontmatter) {
        inFrontmatter = true;
        continue;
      } else break;
    }
    if (!inFrontmatter) continue;
    const match = line.match(/^(\w+):\s*(.+)/);
    if (!match) continue;
    const [, key, value] = match;
    if (key === 'name') name = value.trim();
    else if (key === 'description') description = value.trim();
    else if (key === 'category') category = value.trim();
    else if (key === 'tags') {
      try {
        tags = JSON.parse(value.trim());
      } catch {
        tags = value.split(',').map((t) => t.trim());
      }
    }
  }
  return name ? { name, description, category, tags } : null;
}

export function useImportDialog(refetchSkills: () => void, fetchStats: () => void) {
  const { importSkill } = useSkillStore();

  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importMode, setImportMode] = useState<ImportMode>('url');
  const [importContent, setImportContent] = useState('');
  const [importFilename, setImportFilename] = useState('');
  const [importing, setImporting] = useState(false);
  const [importPreview, setImportPreview] = useState<ImportPreview | null>(null);

  const handleImportContentChange = useCallback(
    (content: string) => {
      setImportContent(content);
      if (importMode === 'paste' || importMode === 'file') {
        setImportPreview(parseFrontmatter(content));
      }
    },
    [importMode],
  );

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setImportFilename(file.name);
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setImportContent(text);
      setImportPreview(parseFrontmatter(text));
    };
    reader.readAsText(file);
  }, []);

  const handleFileDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (!file || !file.name.endsWith('.md')) return;
    setImportFilename(file.name);
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      setImportContent(text);
      setImportPreview(parseFrontmatter(text));
    };
    reader.readAsText(file);
  }, []);

  const handleOpenImportDialog = useCallback(() => {
    setShowImportDialog(true);
    setImportMode('url');
    setImportContent('');
    setImportFilename('');
    setImportPreview(null);
  }, []);

  const handleCloseImportDialog = useCallback(() => {
    setShowImportDialog(false);
  }, []);

  const handleImport = useCallback(async () => {
    if (!importContent.trim()) {
      showError('导入失败', '内容不能为空');
      return;
    }
    setImporting(true);
    const result = await importSkill(importMode, importContent, importFilename || undefined);
    setImporting(false);
    if (result) {
      showSuccess('导入成功', `技能 "${result.name}" 已导入`);
      setShowImportDialog(false);
      setImportContent('');
      setImportFilename('');
      setImportPreview(null);
      refetchSkills();
      fetchStats();
    } else {
      showError('导入失败', useSkillStore.getState().error || '未知错误');
    }
  }, [importMode, importContent, importFilename, importSkill, refetchSkills, fetchStats]);

  return {
    showImportDialog,
    importMode,
    importContent,
    importFilename,
    importing,
    importPreview,
    setImportMode,
    handleImportContentChange,
    handleFileSelect,
    handleFileDrop,
    handleOpenImportDialog,
    handleCloseImportDialog,
    handleImport,
  };
}
