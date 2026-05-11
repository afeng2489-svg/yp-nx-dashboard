import { useSkillStore } from '@/stores/skillStore';
import { useSearchFilter } from './useSearchFilter';
import { useEditDialog } from './useEditDialog';
import { useExecuteDialog } from './useExecuteDialog';
import { useImportDialog } from './useImportDialog';
import { useMultiSelect } from './useMultiSelect';

export function useSkillsPage() {
  const { toggleSkillEnabled } = useSkillStore();

  const search = useSearchFilter();
  const edit = useEditDialog(search.currentSkill, search.refetchSkills, search.fetchStats);
  const execute = useExecuteDialog(search.currentSkill);
  const imp = useImportDialog(search.refetchSkills, search.fetchStats);
  const multi = useMultiSelect(
    search.displaySkills,
    search.currentSkill,
    search.setSelectedSkill,
    search.refetchSkills,
    search.fetchStats,
  );

  return {
    ...search,
    ...edit,
    ...execute,
    ...imp,
    ...multi,
    toggleSkillEnabled,
  };
}
