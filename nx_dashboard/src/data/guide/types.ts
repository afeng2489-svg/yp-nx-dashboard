export type ModuleId =
  | 'dashboard'
  | 'workflows'
  | 'canvas'
  | 'executions'
  | 'sprint-board'
  | 'teams'
  | 'teams-v2'
  | 'roles'
  | 'group-chat'
  | 'processes'
  | 'projects'
  | 'templates'
  | 'skills'
  | 'terminal'
  | 'browser'
  | 'search'
  | 'ui-design'
  | 'tasks'
  | 'cost'
  | 'settings'
  | 'ai-settings'
  | 'editor';

export interface ModuleInfo {
  id: ModuleId;
  label: string;
  group: string;
  purpose: string;
  when: string[];
  relatedTo: ModuleId[];
  tips?: string[];
  path: string;
}

export interface ScenarioStep {
  moduleId: ModuleId;
  action: string;
  detail?: string;
}

export interface Scenario {
  id: string;
  name: string;
  emoji: string;
  description: string;
  highlight?: string;
  steps: ScenarioStep[];
}
