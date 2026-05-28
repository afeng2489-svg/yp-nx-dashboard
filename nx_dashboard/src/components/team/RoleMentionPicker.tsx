import { useMemo, useState } from 'react';
import { cn } from '@/lib/utils';
import type { Role } from '@/stores/teamStore';

export interface RoleMentionPickerProps {
  roles: Role[];
  value: string;
  onChange: (value: string) => void;
  onInsert: (mention: string) => void;
  inputRef?: React.RefObject<HTMLInputElement | null>;
}

/** AF-UX-04a：@ 角色选择器 */
export function RoleMentionPicker({
  roles,
  value,
  onChange,
  onInsert,
  inputRef,
}: RoleMentionPickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);

  const atIndex = value.lastIndexOf('@');
  const active = atIndex >= 0 && !value.slice(atIndex).includes(' ');

  const filtered = useMemo(() => {
    if (!active) return [];
    const q = (query || value.slice(atIndex + 1)).toLowerCase();
    return roles.filter((r) => r.name.toLowerCase().includes(q));
  }, [active, query, roles, value, atIndex]);

  const pick = (role: Role) => {
    const before = value.slice(0, atIndex);
    const mention = `@${role.name} `;
    onInsert(before + mention);
    onChange(before + mention);
    setOpen(false);
    setQuery('');
    inputRef?.current?.focus();
  };

  if (!active || filtered.length === 0) return null;

  return (
    <div
      className="absolute bottom-full left-0 mb-1 w-full max-w-sm rounded-lg border border-border bg-popover shadow-lg z-20 py-1"
      data-testid="role-mention-picker"
    >
      {filtered.map((role, i) => (
        <button
          key={role.id}
          type="button"
          className={cn(
            'w-full text-left px-3 py-2 text-sm hover:bg-accent',
            i === index && 'bg-accent',
          )}
          onMouseDown={(e) => {
            e.preventDefault();
            pick(role);
          }}
          onMouseEnter={() => setIndex(i)}
        >
          <span className="font-medium">@{role.name}</span>
          {role.description && (
            <span className="text-xs text-muted-foreground ml-2">{role.description}</span>
          )}
        </button>
      ))}
    </div>
  );
}

export function parseMentionRoleIds(content: string, roles: Role[]): string[] {
  const ids: string[] = [];
  for (const role of roles) {
    if (content.includes(`@${role.name}`)) ids.push(role.id);
  }
  return ids;
}

export function roleColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  const hues = ['emerald', 'sky', 'violet', 'amber', 'rose', 'teal'];
  return hues[Math.abs(hash) % hues.length];
}
