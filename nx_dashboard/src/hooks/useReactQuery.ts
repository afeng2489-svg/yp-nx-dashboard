import { useQuery, useQueryClient, useMutation, useInfiniteQuery } from '@tanstack/react-query';
import { api } from '@/api/client';
import { useWorkflowStore } from '@/stores/workflowStore';
import { useExecutionStore } from '@/stores/executionStore';
import { useSessionStore } from '@/stores/sessionStore';
import { useTaskStore } from '@/stores/taskStore';
import { useSkillStore } from '@/stores/skillStore';
import { useTeamStore } from '@/stores/teamStore';
import { useWisdomStore } from '@/stores/wisdomStore';
import { useSearchStore } from '@/stores/searchStore';
import type { SearchMode } from '@/types/search';

// ============ Dashboard Combined Query ============
// Use this hook on dashboard for parallel fetching with single loading state

export function useDashboardData() {
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);
  const fetchSessions = useSessionStore((s) => s.fetchSessions);

  return useQuery({
    queryKey: ['dashboard'],
    queryFn: async () => {
      // Fetch all in parallel for faster loading
      await Promise.all([fetchWorkflows(), fetchExecutions(), fetchSessions()]);
      return true;
    },
    staleTime: 1000 * 30, // 30 seconds
    refetchOnMount: true,
  });
}

// ============ Individual Hooks ============

export function useWorkflowsQuery() {
  const fetchWorkflows = useWorkflowStore((s) => s.fetchWorkflows);
  const workflows = useWorkflowStore((s) => s.workflows);

  const query = useQuery({
    queryKey: ['workflows'],
    queryFn: async () => {
      await fetchWorkflows();
      return useWorkflowStore.getState().workflows;
    },
    staleTime: 0, // always re-fetch on mount so newly created workflows show immediately
    refetchInterval: 1000 * 60, // background poll every minute
  });

  return {
    workflows: query.data ?? workflows,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useExecutionsQuery() {
  const fetchExecutions = useExecutionStore((s) => s.fetchExecutions);
  const executions = useExecutionStore((s) => s.executions);

  const query = useQuery({
    queryKey: ['executions'],
    queryFn: async () => {
      await fetchExecutions();
      return useExecutionStore.getState().executions;
    },
    staleTime: 0, // always re-fetch on mount
    refetchInterval: 1000 * 15, // poll every 15s — executions change frequently
  });

  return {
    executions: query.data ?? executions,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useSessionsQuery() {
  const fetchSessions = useSessionStore((s) => s.fetchSessions);
  const sessions = useSessionStore((s) => s.sessions);

  const query = useQuery({
    queryKey: ['sessions'],
    queryFn: async () => {
      await fetchSessions();
      return useSessionStore.getState().sessions;
    },
    staleTime: 0, // always re-fetch on mount
  });

  return {
    sessions: query.data ?? sessions,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useTasksQuery() {
  const fetchTasks = useTaskStore((s) => s.fetchTasks);
  const tasks = useTaskStore((s) => s.tasks);
  const stats = useTaskStore((s) => s.stats);

  const query = useQuery({
    queryKey: ['tasks'],
    queryFn: async () => {
      await fetchTasks();
      const s = useTaskStore.getState();
      return { tasks: s.tasks, stats: s.stats };
    },
    staleTime: 1000 * 10, // 10 seconds for polling
    refetchInterval: 5000, // Auto-refresh every 5 seconds
  });

  return {
    tasks: query.data?.tasks ?? tasks,
    stats: query.data?.stats ?? stats,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

// ============ Skills Query ============

export function useSkillsQuery() {
  const fetchSkills = useSkillStore((s) => s.fetchSkills);
  const skills = useSkillStore((s) => s.skills);

  const query = useQuery({
    queryKey: ['skills'],
    queryFn: async () => {
      await fetchSkills();
      return useSkillStore.getState().skills;
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });

  return {
    skills: query.data ?? skills,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useSkillDetailQuery(skillId: string | null) {
  const fetchSkill = useSkillStore((s) => s.fetchSkill);
  const currentSkill = useSkillStore((s) => s.currentSkill);

  const query = useQuery({
    queryKey: ['skill', skillId],
    queryFn: async () => {
      if (!skillId) return null;
      await fetchSkill(skillId);
      return useSkillStore.getState().currentSkill;
    },
    enabled: !!skillId,
    staleTime: 1000 * 60 * 10, // 10 minutes
  });

  return {
    skill: query.data ?? currentSkill,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useSkillCategoriesQuery() {
  const fetchCategories = useSkillStore((s) => s.fetchCategories);
  const categories = useSkillStore((s) => s.categories);

  const query = useQuery({
    queryKey: ['skill-categories'],
    queryFn: async () => {
      await fetchCategories();
      return useSkillStore.getState().categories;
    },
    staleTime: 1000 * 60 * 10, // 10 minutes
  });

  return {
    categories: query.data ?? categories,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

// ============ Teams Query ============

export function useTeamsQuery() {
  const fetchTeams = useTeamStore((s) => s.fetchTeams);
  const teams = useTeamStore((s) => s.teams);

  const query = useQuery({
    queryKey: ['teams'],
    queryFn: async () => {
      await fetchTeams();
      return useTeamStore.getState().teams;
    },
    staleTime: 0, // always re-fetch on mount so new teams appear immediately
  });

  return {
    teams: query.data ?? teams,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useTeamDetailQuery(teamId: string | null) {
  const getTeam = useTeamStore((s) => s.getTeam);

  const query = useQuery({
    queryKey: ['team', teamId],
    queryFn: async () => {
      if (!teamId) return null;
      return await getTeam(teamId);
    },
    enabled: !!teamId,
    staleTime: 1000 * 60, // 1 minute
  });

  return {
    team: query.data,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useTeamRolesQuery(teamId: string | null) {
  const fetchRoles = useTeamStore((s) => s.fetchRoles);

  const query = useQuery({
    queryKey: ['team-roles', teamId],
    queryFn: async () => {
      if (!teamId) return [];
      await fetchRoles(teamId);
      return useTeamStore.getState().roles[teamId] || [];
    },
    enabled: !!teamId,
    staleTime: 0, // always re-fetch on mount
  });

  return {
    roles: query.data ?? [],
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

// ============ Wisdom Query ============

export function useWisdomEntriesQuery(category?: string | null) {
  const fetchEntries = useWisdomStore((s) => s.fetchEntries);
  const entries = useWisdomStore((s) => s.entries);

  const query = useQuery({
    queryKey: ['wisdom-entries', category],
    queryFn: async () => {
      await fetchEntries(
        category
          ? { category: category as import('@/stores/wisdomStore').WisdomCategory }
          : undefined,
      );
      // Re-read from store after fetch completes to get updated value
      return useWisdomStore.getState().entries;
    },
    staleTime: 0, // always re-fetch on mount
  });

  return {
    entries: query.data ?? entries,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

export function useWisdomCategoriesQuery() {
  const fetchCategories = useWisdomStore((s) => s.fetchCategories);
  const categories = useWisdomStore((s) => s.categories);

  const query = useQuery({
    queryKey: ['wisdom-categories'],
    queryFn: async () => {
      await fetchCategories();
      // Re-read from store after fetch completes to get updated value
      return useWisdomStore.getState().categories;
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });

  return {
    categories: query.data ?? categories,
    loading: query.isLoading,
    refetch: query.refetch,
  };
}

// ============ Search Query ============

export function useSearchQuery(query: string, mode: SearchMode) {
  const search = useSearchStore((s) => s.search);
  const results = useSearchStore((s) => s.results);

  const queryResult = useQuery({
    queryKey: ['search', query, mode],
    queryFn: async () => {
      if (!query.trim()) return null;
      await search(query, mode);
      return useSearchStore.getState().results;
    },
    enabled: query.trim().length > 0,
    staleTime: 1000 * 60, // 1 minute - search results can be cached
  });

  return {
    results: queryResult.data ?? results,
    loading: queryResult.isLoading,
    refetch: queryResult.refetch,
  };
}

// ============ AI Config Query ============

export function useAIProvidersQuery() {
  return useQuery({
    queryKey: ['ai-providers'],
    queryFn: async () => {
      const response = await api.listProviders();
      return response.providers;
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}

export function useAICLIsQuery() {
  return useQuery({
    queryKey: ['ai-clis'],
    queryFn: async () => {
      const response = await api.listCLIs();
      return response;
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}

export function useAIModelsQuery() {
  return useQuery({
    queryKey: ['ai-models'],
    queryFn: async () => {
      return api.listModels();
    },
    staleTime: 1000 * 60 * 5, // 5 minutes
  });
}

export function useAISelectedModelQuery() {
  return useQuery({
    queryKey: ['ai-selected-model'],
    queryFn: async () => {
      return api.getSelectedModel();
    },
    staleTime: 1000 * 30, // 30 seconds
  });
}

export function useAIModelRefreshStatusQuery() {
  return useQuery({
    queryKey: ['ai-model-refresh-status'],
    queryFn: async () => {
      return api.getRefreshStatus();
    },
    staleTime: 1000 * 10, // 10 seconds
    refetchInterval: 10000, // Auto-refresh every 10 seconds
  });
}

export function useAISuggestionMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (prompt: string) => {
      return api.getSuggestion(prompt);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-suggestion'] });
    },
  });
}

export function useAISetSelectedModelMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (modelId: string) => {
      return api.setSelectedModel(modelId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-selected-model'] });
      queryClient.invalidateQueries({ queryKey: ['ai-models'] });
    },
  });
}

export function useAISetDefaultModelMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ modelId, provider }: { modelId: string; provider: string }) => {
      return api.setDefaultModel(modelId, provider);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-models'] });
    },
  });
}

export function useAIRefreshModelsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      return api.refreshModels();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-models'] });
      queryClient.invalidateQueries({ queryKey: ['ai-model-refresh-status'] });
    },
  });
}

export function useAIUpdateCLIStrategyMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ strategy, defaultCli }: { strategy: string; defaultCli?: string }) => {
      return api.updateCLIStrategy(strategy, defaultCli);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-clis'] });
    },
  });
}

export function useAIUpdateCLIConfigMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      cli,
      updates,
    }: {
      cli: string;
      updates: { enabled?: boolean; path?: string };
    }) => {
      return api.updateCLIConfig(cli, updates);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['ai-clis'] });
    },
  });
}
