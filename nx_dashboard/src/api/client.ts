// Use relative URLs - in dev mode Vite proxy handles /api -> http://localhost:8080
// In Tauri production, the API server should be accessible directly
const API_BASE = '';

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  private getUrl(endpoint: string): string {
    // In Tauri or when using Vite proxy, use relative URL
    // Otherwise prepend API_BASE for direct connection
    if (this.baseUrl) {
      return `${this.baseUrl}${endpoint}`;
    }
    return endpoint;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = this.getUrl(endpoint);
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      let errorMessage = `API Error: ${response.status}`;
      try {
        const errorBody = await response.json();
        if (errorBody && typeof errorBody === 'string') {
          errorMessage = errorBody;
        } else if (errorBody?.error) {
          errorMessage = errorBody.error;
        } else if (errorBody?.message) {
          errorMessage = errorBody.message;
        }
      } catch {
        // If we can't parse error body, use the status text
        if (response.statusText) {
          errorMessage = response.statusText;
        }
      }
      throw new Error(errorMessage);
    }

    return response.json();
  }

  async health() {
    return this.request<{ status: string; version: string }>('/health');
  }

  async listWorkflows() {
    return this.request<Workflow[]>('/api/v1/workflows');
  }

  async getWorkflow(id: string) {
    return this.request<Workflow>(`/api/v1/workflows/${id}`);
  }

  async createWorkflow(workflow: CreateWorkflowRequest) {
    return this.request<Workflow>('/api/v1/workflows', {
      method: 'POST',
      body: JSON.stringify(workflow),
    });
  }

  async updateWorkflow(id: string, updates: Partial<Workflow>) {
    return this.request<Workflow>(`/api/v1/workflows/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deleteWorkflow(id: string) {
    return this.request<void>(`/api/v1/workflows/${id}`, {
      method: 'DELETE',
    });
  }

  async executeWorkflow(id: string, variables?: Record<string, unknown>) {
    return this.request<Execution>(`/api/v1/workflows/${id}/execute`, {
      method: 'POST',
      body: JSON.stringify({ variables }),
    });
  }

  async listExecutions() {
    return this.request<Execution[]>('/api/v1/executions');
  }

  async getExecution(id: string) {
    return this.request<Execution>(`/api/v1/executions/${id}`);
  }

  async cancelExecution(id: string) {
    return this.request<void>(`/api/v1/executions/${id}/cancel`, {
      method: 'POST',
    });
  }

  async listSessions() {
    return this.request<Session[]>('/api/v1/sessions');
  }

  async getSession(id: string) {
    return this.request<Session>(`/api/v1/sessions/${id}`);
  }

  async createSession(workflowId: string) {
    return this.request<Session>('/api/v1/sessions', {
      method: 'POST',
      body: JSON.stringify({ workflow_id: workflowId }),
    });
  }

  async deleteSession(id: string) {
    return this.request<void>(`/api/v1/sessions/${id}`, {
      method: 'DELETE',
    });
  }

  // AI Model Selection APIs
  async listModels() {
    return this.request<ModelInfo[]>('/api/v1/ai/models');
  }

  async getSelectedModel() {
    return this.request<SelectedModelResponse>('/api/v1/ai/selected');
  }

  async getClaudeCliModel() {
    return this.request<ClaudeCliModelResponse>('/api/v1/ai/cli-model');
  }

  async setSelectedModel(modelId: string) {
    return this.request<{ success: boolean; model_id: string }>('/api/v1/ai/selected', {
      method: 'PUT',
      body: JSON.stringify({ model_id: modelId }),
    });
  }

  async setDefaultModel(modelId: string, provider: string) {
    return this.request<{ success: boolean; model_id: string }>('/api/v1/ai/default', {
      method: 'PUT',
      body: JSON.stringify({ model_id: modelId, provider }),
    });
  }

  async getProviderModels(provider: string) {
    return this.request<ModelInfo[]>(`/api/v1/ai/providers/${provider}/models`);
  }

  async updateModelConfig(modelId: string, maxTokens?: number, temperature?: number) {
    return this.request<{ success: boolean }>('/api/v1/ai/models/config', {
      method: 'PUT',
      body: JSON.stringify({ model_id: modelId, max_tokens: maxTokens, temperature }),
    });
  }

  // AI Provider APIs
  async listProviders() {
    return this.request<{ providers: ProviderInfo[] }>('/api/v1/ai/providers');
  }

  async listCLIs() {
    return this.request<CLIListResponse>('/api/v1/ai/clis');
  }

  async updateCLIStrategy(strategy: string, defaultCli?: string) {
    return this.request<{ success: boolean }>('/api/v1/ai/strategy', {
      method: 'PUT',
      body: JSON.stringify({ strategy, default_cli: defaultCli }),
    });
  }

  async updateCLIConfig(cli: string, updates: { enabled?: boolean; path?: string }) {
    return this.request<{ success: boolean }>('/api/v1/ai/clis/config', {
      method: 'PUT',
      body: JSON.stringify({ cli, ...updates }),
    });
  }

  async getSuggestion(prompt: string) {
    return this.request<SelectionSuggestion>('/api/v1/ai/suggestion', {
      method: 'POST',
      body: JSON.stringify(prompt),
    });
  }

  async getRefreshStatus() {
    return this.request<ModelRefreshStatus>('/api/v1/ai/models/refresh-status');
  }

  async refreshModels() {
    return this.request<RefreshModelsResponse>('/api/v1/ai/models/refresh', {
      method: 'POST',
    });
  }

  // Execute CLI command
  async executeCLI(request: {
    prompt: string;
    cli?: string;
    working_directory?: string;
    timeout_secs?: number;
    auto_yes?: boolean;
  }) {
    return this.request<ExecuteCLIResponse>('/api/v1/ai/execute', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  // API Key management
  async listApiKeys() {
    return this.request<ApiKeyInfo[]>('/api/v1/ai/api-keys');
  }

  async saveApiKey(provider: string, apiKey: string) {
    return this.request<{ success: boolean; provider: string }>('/api/v1/ai/api-keys', {
      method: 'POST',
      body: JSON.stringify({ provider, api_key: apiKey }),
    });
  }

  async deleteApiKey(provider: string) {
    return this.request<{ success: boolean }>(`/api/v1/ai/api-keys/${provider}`, {
      method: 'DELETE',
    });
  }

  // AI Provider V2 APIs
  async listProvidersV2() {
    return this.request<{ providers: AIProvider[] }>('/api/v1/ai/v2/providers');
  }

  async getProviderV2(id: string) {
    return this.request<AIProvider>(`/api/v1/ai/v2/providers/${id}`);
  }

  async createProviderV2(provider: CreateProviderRequest) {
    return this.request<AIProvider>('/api/v1/ai/v2/providers', {
      method: 'POST',
      body: JSON.stringify(provider),
    });
  }

  async updateProviderV2(id: string, updates: UpdateProviderRequest) {
    return this.request<AIProvider>(`/api/v1/ai/v2/providers/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deleteProviderV2(id: string) {
    return this.request<{ success: boolean }>(`/api/v1/ai/v2/providers/${id}`, {
      method: 'DELETE',
    });
  }

  async getProviderMappings(providerId: string) {
    return this.request<ModelMapping[]>(`/api/v1/ai/v2/providers/${providerId}/models`);
  }

  async addModelMapping(providerId: string, mapping: AddModelMappingRequest) {
    return this.request<{ success: boolean }>(`/api/v1/ai/v2/providers/${providerId}/models`, {
      method: 'POST',
      body: JSON.stringify(mapping),
    });
  }

  async removeModelMapping(providerId: string, mappingId: string) {
    return this.request<{ success: boolean }>(
      `/api/v1/ai/v2/providers/${providerId}/models/${mappingId}/main`,
      {
        method: 'DELETE',
      },
    );
  }

  async testProviderConnection(providerId: string) {
    return this.request<ConnectionTestResult>(
      `/api/v1/ai/v2/providers/${providerId}/test-connection`,
      {
        method: 'POST',
      },
    );
  }

  async enableProvider(providerId: string, model?: string) {
    return this.request<{ success: boolean; message: string; provider_id: string; model: string }>(
      `/api/v1/ai/v2/providers/${providerId}/enable`,
      {
        method: 'POST',
        body: JSON.stringify({ model }),
      },
    );
  }

  async disableProvider(providerId: string) {
    return this.request<{ success: boolean; message: string; model: string }>(
      `/api/v1/ai/v2/providers/${providerId}/disable`,
      {
        method: 'POST',
      },
    );
  }

  async getProviderPresets() {
    return this.request<{ presets: ProviderPreset[] }>('/api/v1/ai/v2/presets');
  }

  async createFromPreset(presetKey: string, apiKey: string) {
    return this.request<AIProvider>('/api/v1/ai/v2/providers/from-preset', {
      method: 'POST',
      body: JSON.stringify({ preset_key: presetKey, api_key: apiKey }),
    });
  }

  // Claude Switch APIs
  async configureClaudeSwitch(backends: ClaudeSwitchBackendConfig[]) {
    return this.request<{ success: boolean; message: string }>(
      '/api/v1/ai/claude-switch/configure',
      {
        method: 'POST',
        body: JSON.stringify({ backends }),
      },
    );
  }

  async listClaudeSwitchBackends() {
    return this.request<ClaudeSwitchBackendInfo[]>('/api/v1/ai/claude-switch/backends');
  }

  async addClaudeSwitchBackend(backend: ClaudeSwitchBackendConfig) {
    return this.request<{ success: boolean; message: string }>(
      '/api/v1/ai/claude-switch/backends',
      {
        method: 'POST',
        body: JSON.stringify(backend),
      },
    );
  }

  async switchClaudeSwitchBackend(backend: string) {
    return this.request<{ success: boolean; message: string }>(
      '/api/v1/ai/claude-switch/backends/switch',
      {
        method: 'POST',
        body: JSON.stringify({ backend }),
      },
    );
  }

  async getActiveClaudeSwitchBackend() {
    return this.request<ClaudeSwitchBackendInfo>('/api/v1/ai/claude-switch/active');
  }

  async testClaudeSwitchBackend(backend: string, apiKey: string, model: string) {
    return this.request<ConnectionTestResult>('/api/v1/ai/claude-switch/backends/test', {
      method: 'POST',
      body: JSON.stringify({ backend, api_key: apiKey, model }),
    });
  }
}

export interface ExecuteCLIResponse {
  output: string;
  error?: string;
  exit_code: number;
  execution_time_ms: number;
  cli: string;
}

export interface ExecuteCLIResponse {
  output: string;
  error?: string;
  exit_code: number;
  execution_time_ms: number;
  cli: string;
}

export interface ApiKeyInfo {
  provider: string;
  has_key: boolean;
  updated_at: string | null;
}

export interface ModelInfo {
  model_id: string;
  provider: string;
  display_name: string;
  description: string;
  supports_chat: boolean;
  supports_completion: boolean;
  is_default: boolean;
}

export interface SelectedModelResponse {
  model_id: string;
  provider: string;
  display_name: string;
}

export interface ClaudeCliModelResponse {
  sonnet_model: string;
  haiku_model: string;
  opus_model: string;
  base_url: string | null;
}

export interface ProviderInfo {
  name: string;
  provider_type: string;
  models: string[];
  supported_clis: string[];
  default_model: string;
}

export interface CLIInfo {
  cli: string;
  display_name: string;
  enabled: boolean;
  available: boolean;
  capability?: {
    cli: string;
    available: boolean;
    version?: string;
    features: string[];
  };
  path?: string;
}

export interface CLIListResponse {
  clis: CLIInfo[];
  selection_strategy: string;
  default_cli?: string;
}

export interface SelectionSuggestion {
  recommended_cli: string;
  reason: string;
  alternatives: string[];
}

export interface ModelRefreshStatus {
  needs_refresh: boolean;
  seconds_until_refresh: number;
  last_refresh_time: string;
}

export interface RefreshModelsResponse {
  success: boolean;
  models_before: number;
  models_after: number;
  message: string;
}

// AI Provider V2 Types
export type APIFormat = 'openai' | 'anthropic' | { custom: string };

export type MappingType = 'main' | 'thinking' | 'haiku' | 'sonnet' | 'opus';

export interface ModelMapping {
  id: string;
  provider_id: string;
  mapping_type: MappingType;
  model_id: string;
  display_name?: string;
  config_json?: string;
}

export interface AIProvider {
  id: string;
  provider_key: string;
  name: string;
  description?: string;
  website?: string;
  base_url: string;
  api_format: APIFormat;
  auth_field: string;
  enabled: boolean;
  config_json?: string;
  created_at: string;
  updated_at: string;
}

export interface ProviderPreset {
  key: string;
  name: string;
  description: string;
  website: string;
  base_url: string;
  api_format: APIFormat;
  default_auth_field: string;
}

export interface CreateProviderRequest {
  name: string;
  provider_key: string;
  description?: string;
  website?: string;
  api_format: APIFormat;
  auth_field: string;
  base_url: string;
  api_key: string;
  enabled?: boolean;
  config_json?: string;
}

export interface UpdateProviderRequest {
  name?: string;
  provider_key?: string;
  description?: string;
  website?: string;
  api_format?: APIFormat;
  auth_field?: string;
  base_url?: string;
  api_key?: string;
  enabled?: boolean;
  config_json?: string;
}

export interface AddModelMappingRequest {
  mapping_type: MappingType;
  model_id: string;
  display_name?: string;
  config_json?: string;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
  models?: string[];
}

// Claude Switch types
export interface ClaudeSwitchBackendInfo {
  backend: string;
  model: string;
  base_url: string;
  is_active: boolean;
}

export interface ClaudeSwitchBackendConfig {
  backend: string;
  api_key: string;
  base_url?: string;
  model: string;
}

export interface Workflow {
  id: string;
  name: string;
  version: string;
  description?: string;
  definition: unknown;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkflowRequest {
  name: string;
  version?: string;
  description?: string;
  definition: unknown;
}

export interface Execution {
  id: string;
  workflow_id: string;
  status: string;
  variables: Record<string, unknown>;
  stage_results: StageResult[];
  started_at?: string;
  finished_at?: string;
  error?: string;
}

export interface StageResult {
  stage_name: string;
  outputs: unknown[];
  completed_at?: string;
}

export interface Session {
  id: string;
  workflow_id?: string;
  status: string;
  resume_key?: string;
  created_at: string;
  updated_at: string;
}

export const api = new ApiClient(API_BASE);
