export type LogEntry = { type: 'system' | 'output' | 'stage' | 'error'; text: string };

export type PauseState = {
  stage_name: string;
  question: string;
  options: { label: string; value: string }[];
};

export interface CommitInfo {
  hash: string;
  full_hash: string;
  message: string;
  timestamp: string;
  changed_files: number;
}

export interface BranchInfo {
  current_branch: string | null;
  exec_branch: string;
  is_git_repo: boolean;
}
