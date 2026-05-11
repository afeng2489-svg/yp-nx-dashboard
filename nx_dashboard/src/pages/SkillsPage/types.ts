import type {
  CreateSkillRequest,
  SkillDetail,
  SkillParameter,
  SkillSummary,
} from '@/stores/skillStore';

// ── Import preview ──────────────────────────────────────────────────────────

export interface ImportPreview {
  name: string;
  description: string;
  category: string;
  tags: string[];
}

export type ImportMode = 'url' | 'file' | 'paste';

// ── Shared input class ──────────────────────────────────────────────────────

export const INPUT_CLS =
  'w-full px-3 py-2 border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary bg-background text-foreground';

// ── Component prop interfaces ───────────────────────────────────────────────

export interface SkillCardProps {
  skill: SkillSummary;
  multiSelectMode: boolean;
  isSelected: boolean;
  selectedSkillId: string | undefined;
  onSkillClick: (skill: SkillSummary) => void;
  onToggleSelect: (id: string) => void;
}

export interface SkillDetailPanelProps {
  skill: SkillDetail;
  executing: boolean;
  onOpenEditDialog: () => void;
  onDelete: () => void;
  onToggleEnabled: (id: string, enabled: boolean) => void;
  onOpenExecuteDialog: () => void;
}

export interface SkillEditDialogProps {
  isCreating: boolean;
  editForm: CreateSkillRequest;
  saving: boolean;
  inputCls: string;
  onFormChange: (form: CreateSkillRequest) => void;
  onClose: () => void;
  onSave: () => void;
}

export interface SkillExecuteDialogProps {
  skillName: string;
  parameters: SkillParameter[];
  paramValues: Record<string, string>;
  executionResult: string | null;
  executing: boolean;
  inputCls: string;
  onParamChange: (name: string, value: string) => void;
  onClose: () => void;
  onExecute: () => void;
}

export interface SkillImportDialogProps {
  importMode: ImportMode;
  importContent: string;
  importFilename: string;
  importing: boolean;
  importPreview: ImportPreview | null;
  inputCls: string;
  onModeChange: (mode: ImportMode) => void;
  onContentChange: (content: string) => void;
  onFileSelect: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onFileDrop: (e: React.DragEvent) => void;
  onClose: () => void;
  onImport: () => void;
}
