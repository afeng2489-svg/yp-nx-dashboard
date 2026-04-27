import { create } from 'zustand';
import {
  api,
  AIProvider,
  ModelMapping,
  ProviderPreset,
  APIFormat,
  MappingType,
  CreateProviderRequest,
  UpdateProviderRequest,
  AddModelMappingRequest,
  ConnectionTestResult,
  ClaudeSwitchBackendInfo,
  ClaudeSwitchBackendConfig,
} from '@/api/client';

// CLI types
export type CLI = 'claude' | 'gemini' | 'codex' | 'qwen' | 'opencode';

export type CLISelectionStrategy = 'auto' | 'semantic' | 'manual' | 'fallback';

// CLI capability info
export interface CLICapability {
  cli: CLI;
  available: boolean;
  version?: string;
  features: string[];
}

// CLI configuration
export interface CLIConfig {
  cli: CLI;
  display_name: string;
  enabled: boolean;
  available: boolean;
  capability?: CLICapability;
  path?: string;
}

// Provider info
export interface ProviderInfo {
  name: string;
  provider_type: string;
  models: string[];
  supported_clis: string[];
  default_model: string;
}

// Selection suggestion
export interface SelectionSuggestion {
  recommended_cli: CLI;
  reason: string;
  alternatives: CLI[];
}

// Execute CLI request
export interface ExecuteCLIRequest {
  prompt: string;
  cli?: CLI;
  working_directory?: string;
  timeout_secs?: number;
}

// Execute CLI response
export interface ExecuteCLIResponse {
  output: string;
  error?: string;
  exit_code: number;
  execution_time_ms: number;
  cli: CLI;
}

// Model info
export interface ModelInfo {
  model_id: string;
  provider: string;
  display_name: string;
  description: string;
  supports_chat: boolean;
  supports_completion: boolean;
  is_default: boolean;
}

// Selected model response
export interface SelectedModelResponse {
  model_id: string;
  provider: string;
  display_name: string;
}

// Model refresh status
export interface ModelRefreshStatus {
  needs_refresh: boolean;
  seconds_until_refresh: number;
  last_refresh_time: string;
}

// Claude Switch types (re-exported from api client)
export type { ClaudeSwitchBackendInfo, ClaudeSwitchBackendConfig } from '@/api/client';

// AI Config Store
interface AIConfigState {
  // Provider list
  providers: ProviderInfo[];
  providersLoading: boolean;
  providersError: string | null;

  // CLI list
  clis: CLIConfig[];
  clisLoading: boolean;
  clisError: string | null;

  // Selection strategy
  selectionStrategy: CLISelectionStrategy;
  defaultCLI: CLI;

  // Current CLI selection
  selectedCLI: CLI;
  manualOverride: boolean;

  // Selection suggestion
  suggestion: SelectionSuggestion | null;
  suggestionLoading: boolean;

  // Execute state
  executing: boolean;
  lastExecution: ExecuteCLIResponse | null;
  executionError: string | null;

  // Model selection (Global Model Selector)
  models: ModelInfo[];
  modelsLoading: boolean;
  modelsError: string | null;
  selectedModel: SelectedModelResponse | null;
  refreshStatus: ModelRefreshStatus | null;
  refreshLoading: boolean;

  // Provider V2 state
  providersV2: AIProvider[];
  providersV2Loading: boolean;
  providersV2Error: string | null;
  selectedProvider: AIProvider | null;
  presets: ProviderPreset[];
  presetsLoading: boolean;

  // Model mappings state
  mappings: Record<string, ModelMapping[]>;
  mappingsLoading: boolean;

  // Actions
  fetchProviders: () => Promise<void>;
  fetchCLIs: () => Promise<void>;
  setSelectionStrategy: (strategy: CLISelectionStrategy) => void;
  setDefaultCLI: (cli: CLI) => void;
  selectCLI: (cli: CLI, manual?: boolean) => void;
  getSuggestion: (prompt: string) => Promise<void>;
  executeCLI: (request: ExecuteCLIRequest) => Promise<ExecuteCLIResponse | null>;
  updateCLIConfig: (cli: CLI, updates: Partial<CLIConfig>) => Promise<void>;
  fetchModels: () => Promise<void>;
  fetchSelectedModel: () => Promise<void>;
  setSelectedModel: (modelId: string) => Promise<void>;
  setDefaultModel: (modelId: string, provider: string) => Promise<void>;
  fetchRefreshStatus: () => Promise<void>;
  refreshModels: () => Promise<void>;

  // Provider V2 actions
  fetchProvidersV2: () => Promise<void>;
  fetchPresets: () => Promise<void>;
  createProvider: (provider: CreateProviderRequest) => Promise<AIProvider | null>;
  updateProvider: (id: string, updates: UpdateProviderRequest) => Promise<AIProvider | null>;
  deleteProvider: (id: string) => Promise<boolean>;
  selectProvider: (provider: AIProvider | null) => void;
  createFromPreset: (presetKey: string, apiKey: string) => Promise<AIProvider | null>;

  // Model mapping actions
  fetchMappings: (providerId: string) => Promise<void>;
  addMapping: (providerId: string, mapping: AddModelMappingRequest) => Promise<void>;
  removeMapping: (providerId: string, mappingId: string) => Promise<void>;
  testConnection: (providerId: string) => Promise<ConnectionTestResult>;
  enableProvider: (
    providerId: string,
    model?: string,
  ) => Promise<{ success: boolean; message: string; model: string } | null>;
  disableProvider: (
    providerId: string,
  ) => Promise<{ success: boolean; message: string; model: string } | null>;

  reset: () => void;
}

const initialState = {
  providers: [] as ProviderInfo[],
  providersLoading: false,
  providersError: null as string | null,
  clis: [] as CLIConfig[],
  clisLoading: false,
  clisError: null as string | null,
  selectionStrategy: 'auto' as CLISelectionStrategy,
  defaultCLI: 'claude' as CLI,
  selectedCLI: 'claude' as CLI,
  manualOverride: false,
  suggestion: null as SelectionSuggestion | null,
  suggestionLoading: false,
  executing: false,
  lastExecution: null as ExecuteCLIResponse | null,
  executionError: null as string | null,
  models: [] as ModelInfo[],
  modelsLoading: false,
  modelsError: null as string | null,
  selectedModel: null as SelectedModelResponse | null,
  refreshStatus: null as ModelRefreshStatus | null,
  refreshLoading: false,
  providersV2: [] as AIProvider[],
  providersV2Loading: false,
  providersV2Error: null as string | null,
  selectedProvider: null as AIProvider | null,
  presets: [] as ProviderPreset[],
  presetsLoading: false,
  mappings: {} as Record<string, ModelMapping[]>,
  mappingsLoading: false,
};

export const useAIConfigStore = create<AIConfigState>((set, get) => {
  return {
    ...initialState,

    // Fetch provider list
    fetchProviders: async () => {
      set({ providersLoading: true, providersError: null });
      try {
        const data = await api.listProviders();
        set({ providers: data.providers, providersLoading: false });
      } catch (error) {
        set({
          providersError: error instanceof Error ? error.message : 'Unknown error',
          providersLoading: false,
        });
      }
    },

    // Fetch CLI list
    fetchCLIs: async () => {
      set({ clisLoading: true, clisError: null });
      try {
        const data = await api.listCLIs();
        set({
          clis: data.clis as CLIConfig[],
          selectionStrategy: data.selection_strategy as CLISelectionStrategy,
          defaultCLI: (data.default_cli as CLI) || 'claude',
          clisLoading: false,
        });
      } catch (error) {
        set({
          clisError: error instanceof Error ? error.message : 'Unknown error',
          clisLoading: false,
        });
      }
    },

    // Set selection strategy
    setSelectionStrategy: (strategy) => {
      set({ selectionStrategy: strategy });
      if (strategy === 'manual') {
        set({ manualOverride: true });
      } else {
        set({ manualOverride: false });
      }
    },

    // Set default CLI
    setDefaultCLI: (cli) => {
      set({ defaultCLI: cli });
    },

    // Select a CLI
    selectCLI: (cli, manual = true) => {
      set({
        selectedCLI: cli,
        manualOverride: manual,
        suggestion: null,
      });
    },

    // Get selection suggestion for a prompt
    getSuggestion: async (prompt) => {
      set({ suggestionLoading: true });
      try {
        const data = await api.getSuggestion(prompt);
        set({
          suggestion: {
            recommended_cli: data.recommended_cli as CLI,
            reason: data.reason,
            alternatives: data.alternatives as CLI[],
          },
          suggestionLoading: false,
        });

        // Auto-select if not in manual mode
        const state = get();
        if (!state.manualOverride) {
          set({ selectedCLI: data.recommended_cli as CLI });
        }
      } catch (error) {
        set({ suggestionLoading: false });
      }
    },

    // Execute CLI command
    executeCLI: async (request) => {
      set({ executing: true, executionError: null });
      try {
        const cli = request.cli || get().selectedCLI;
        const data = await api.executeCLI({ ...request, cli });
        set({
          lastExecution: data as ExecuteCLIResponse,
          executing: false,
        });
        return data as ExecuteCLIResponse;
      } catch (error) {
        set({
          executionError: error instanceof Error ? error.message : 'Unknown error',
          executing: false,
        });
        return null;
      }
    },

    // Update CLI config
    updateCLIConfig: async (cli, updates) => {
      try {
        await api.updateCLIConfig(cli, updates);
        // Refresh CLI list
        await get().fetchCLIs();
      } catch (error) {
        console.error('Failed to update CLI config:', error);
      }
    },

    // Fetch available models
    fetchModels: async () => {
      set({ modelsLoading: true, modelsError: null });
      try {
        const data = await api.listModels();
        set({ models: data, modelsLoading: false });
      } catch (error) {
        set({
          modelsError: error instanceof Error ? error.message : 'Unknown error',
          modelsLoading: false,
        });
      }
    },

    // Fetch currently selected model
    fetchSelectedModel: async () => {
      try {
        const data = await api.getSelectedModel();
        set({ selectedModel: data });
      } catch (error) {
        console.error('Failed to fetch selected model:', error);
      }
    },

    // Set selected model
    setSelectedModel: async (modelId) => {
      try {
        await api.setSelectedModel(modelId);
        // Refresh selected model
        await get().fetchSelectedModel();
      } catch (error) {
        console.error('Failed to set selected model:', error);
      }
    },

    // Set default model for a provider
    setDefaultModel: async (modelId: string, provider: string) => {
      try {
        await api.setDefaultModel(modelId, provider);
        // Refresh models list
        await get().fetchModels();
      } catch (error) {
        console.error('Failed to set default model:', error);
      }
    },

    // Fetch model refresh status
    fetchRefreshStatus: async () => {
      try {
        const data = await api.getRefreshStatus();
        set({ refreshStatus: data });
      } catch (error) {
        console.error('Failed to fetch refresh status:', error);
      }
    },

    // Manually refresh models
    refreshModels: async () => {
      set({ refreshLoading: true });
      try {
        await api.refreshModels();
        // Refresh models list and status
        await get().fetchModels();
        await get().fetchRefreshStatus();
        set({ refreshLoading: false });
      } catch (error) {
        console.error('Failed to refresh models:', error);
        set({ refreshLoading: false });
      }
    },

    // Fetch Provider V2 list
    fetchProvidersV2: async () => {
      set({ providersV2Loading: true, providersV2Error: null });
      try {
        const data = await api.listProvidersV2();
        set({ providersV2: data.providers, providersV2Loading: false });
      } catch (error) {
        set({
          providersV2Error: error instanceof Error ? error.message : 'Unknown error',
          providersV2Loading: false,
        });
      }
    },

    // Fetch presets
    fetchPresets: async () => {
      set({ presetsLoading: true });
      try {
        const data = await api.getProviderPresets();
        set({ presets: data.presets, presetsLoading: false });
      } catch (error) {
        console.error('Failed to fetch presets:', error);
        set({ presetsLoading: false });
      }
    },

    // Create provider
    createProvider: async (provider) => {
      try {
        const created = await api.createProviderV2(provider);
        await get().fetchProvidersV2();
        return created;
      } catch (error) {
        console.error('Failed to create provider:', error);
        return null;
      }
    },

    // Update provider
    updateProvider: async (id, updates) => {
      try {
        const updated = await api.updateProviderV2(id, updates);
        await get().fetchProvidersV2();
        return updated;
      } catch (error) {
        console.error('Failed to update provider:', error);
        return null;
      }
    },

    // Delete provider
    deleteProvider: async (id) => {
      try {
        await api.deleteProviderV2(id);
        await get().fetchProvidersV2();
        return true;
      } catch (error) {
        console.error('Failed to delete provider:', error);
        return false;
      }
    },

    // Select provider
    selectProvider: (provider) => {
      set({ selectedProvider: provider });
    },

    // Create from preset
    createFromPreset: async (presetKey, apiKey) => {
      try {
        const created = await api.createFromPreset(presetKey, apiKey);
        await get().fetchProvidersV2();
        return created;
      } catch (error) {
        console.error('Failed to create from preset:', error);
        return null;
      }
    },

    // Fetch model mappings for a provider
    fetchMappings: async (providerId) => {
      console.log('[Store] fetchMappings called for provider:', providerId);
      set({ mappingsLoading: true });
      try {
        const data = await api.getProviderMappings(providerId);
        console.log('[Store] fetchMappings received data:', data);
        set((prevState) => ({
          mappings: { ...prevState.mappings, [providerId]: data },
          mappingsLoading: false,
        }));
        console.log('[Store] mappings state updated');
      } catch (error) {
        console.error('[Store] Failed to fetch mappings:', error);
        set({ mappingsLoading: false });
      }
    },

    // Add model mapping
    addMapping: async (providerId, mapping) => {
      console.log('[Store] addMapping called:', providerId, mapping);
      try {
        const result = await api.addModelMapping(providerId, mapping);
        console.log('[Store] addModelMapping result:', result);
        await get().fetchMappings(providerId);
      } catch (error) {
        console.error('[Store] Failed to add mapping:', error);
        throw error; // Re-throw to let UI handle it
      }
    },

    // Remove model mapping
    removeMapping: async (providerId, mappingId) => {
      try {
        await api.removeModelMapping(providerId, mappingId);
        await get().fetchMappings(providerId);
      } catch (error) {
        console.error('Failed to remove mapping:', error);
      }
    },

    // Test provider connection
    testConnection: async (providerId: string): Promise<ConnectionTestResult> => {
      console.log('[Store] testing connection for provider:', providerId);
      const result = await api.testProviderConnection(providerId);
      console.log('[Store] connection test result:', result);
      return result;
    },

    // Enable provider as default AI
    enableProvider: async (providerId: string, model?: string) => {
      const result = await api.enableProvider(providerId, model);
      console.log('[Store] enable provider result:', result);
      if (result?.success) {
        // Update the selected model state to reflect the change
        const selectedProvider = get().providersV2.find((p) => p.id === providerId);
        if (selectedProvider && result.model) {
          set({
            selectedModel: {
              model_id: `claude-switch-${selectedProvider.provider_key}`,
              provider: selectedProvider.provider_key,
              display_name: result.model,
            },
          });
        }
      }
      return result;
    },

    // Disable provider (switch back to default Claude)
    disableProvider: async (providerId: string) => {
      const result = await api.disableProvider(providerId);
      console.log('[Store] disable provider result:', result);
      if (result?.success) {
        // Update selected model back to default
        set({
          selectedModel: {
            model_id: 'claude-sonnet-4-5',
            provider: 'anthropic',
            display_name: 'Claude Sonnet 4.5',
          },
        });
      }
      return result;
    },

    // Reset state
    reset: () => {
      set(initialState);
    },
  };
});

// Utility functions
export const getCLIDisplayName = (cli: CLI): string => {
  const names: Record<CLI, string> = {
    claude: 'Claude',
    gemini: 'Gemini',
    codex: 'Codex',
    qwen: 'Qwen',
    opencode: 'OpenCode',
  };
  return names[cli] || cli;
};

export const getCLIFeatureList = (cli: CLI): string[] => {
  const features: Record<CLI, string[]> = {
    claude: ['Code Review', 'Debugging', 'Explanation', 'Refactoring'],
    gemini: ['Multimodal', 'Long Context', 'Creative Tasks'],
    codex: ['Code Generation', 'Algorithm Implementation', 'Function Writing'],
    qwen: ['Chinese Language', 'Math Reasoning', 'Logic'],
    opencode: ['Open Source Projects', 'GitHub Integration', 'Popular Frameworks'],
  };
  return features[cli] || [];
};

export const getStrategyDisplayName = (strategy: CLISelectionStrategy): string => {
  const names: Record<CLISelectionStrategy, string> = {
    auto: 'Auto',
    semantic: 'Semantic',
    manual: 'Manual',
    fallback: 'Fallback',
  };
  return names[strategy] || strategy;
};
