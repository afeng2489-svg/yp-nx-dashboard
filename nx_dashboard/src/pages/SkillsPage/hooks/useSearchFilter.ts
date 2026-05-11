import { useEffect, useState, useCallback } from 'react';
import { useSkillStore, type SkillSummary } from '@/stores/skillStore';
import {
  useSkillsQuery,
  useSkillDetailQuery,
  useSkillCategoriesQuery,
} from '@/hooks/useReactQuery';

export function useSearchFilter() {
  const { currentSkill, searchResults, stats, error, searchSkills, fetchStats, clearSearch } =
    useSkillStore();

  const { skills, loading: skillsLoading, refetch: refetchSkills } = useSkillsQuery();
  const { categories, loading: categoriesLoading } = useSkillCategoriesQuery();
  useSkillDetailQuery(currentSkill?.id || null);
  const filteredSkills = useSkillStore((s) => s.skills);

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedSkill, setSelectedSkill] = useState<SkillSummary | null>(null);

  const isLoading = skillsLoading || categoriesLoading;

  const displaySkills = searchQuery.trim()
    ? searchResults
    : selectedCategory
      ? filteredSkills
      : skills;

  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  useEffect(() => {
    if (currentSkill && selectedSkill?.id === currentSkill.id) {
      // Already have this skill
    } else if (selectedSkill) {
      // Fetch skill detail if we only have summary
    }
  }, [selectedSkill, currentSkill]);

  const handleSearch = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (searchQuery.trim()) {
        searchSkills(searchQuery);
        setSelectedCategory(null);
      }
    },
    [searchQuery, searchSkills],
  );

  const handleCategoryClick = useCallback(
    (category: string) => {
      if (selectedCategory === category) {
        setSelectedCategory('');
        setSearchQuery('');
        clearSearch();
        refetchSkills();
      } else {
        setSelectedCategory(category);
        setSearchQuery('');
        clearSearch();
        useSkillStore.getState().fetchByCategory(category);
      }
    },
    [selectedCategory, clearSearch, refetchSkills],
  );

  const handleSkillClick = useCallback(async (skill: SkillSummary) => {
    setSelectedSkill(skill);
    const detail = await useSkillStore.getState().fetchSkill(skill.id);
    if (detail) {
      setSelectedSkill(skill);
    }
  }, []);

  return {
    currentSkill,
    displaySkills,
    categories,
    stats,
    error,
    isLoading,
    searchQuery,
    setSearchQuery,
    handleSearch,
    selectedCategory,
    handleCategoryClick,
    selectedSkill,
    setSelectedSkill,
    handleSkillClick,
    refetchSkills,
    fetchStats,
  };
}
