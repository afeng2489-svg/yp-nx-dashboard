import { useState, useRef, useEffect } from 'react';
import { Send, Sparkles } from 'lucide-react';
import { cn } from '@/lib/utils';
import { SkillSummary } from '@/stores/skillStore';
import { RoleMentionPicker } from '@/components/team/RoleMentionPicker';
import { isP5TeamChatAtEnabled } from '@/data/factoryFeatureFlags';
import type { Role } from '@/stores/teamStore';

export interface ChatInputProps {
  onSend: (message: string) => void;
  disabled: boolean;
  skills: SkillSummary[];
  roles?: Role[];
}

export function ChatInput({ onSend, disabled, skills, roles = [] }: ChatInputProps) {
  const [newMessage, setNewMessage] = useState('');
  const [showSkillHint, setShowSkillHint] = useState(false);
  const [skillSearch, setSkillSearch] = useState('');
  const [selectedSkillIndex, setSelectedSkillIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const skillHintRef = useRef<HTMLDivElement>(null);
  const isComposingRef = useRef(false);
  const atEnabled = isP5TeamChatAtEnabled();

  // Filter skills based on search
  const filteredSkills = skillSearch
    ? skills.filter(
        (s) =>
          s.name.toLowerCase().includes(skillSearch.toLowerCase()) ||
          s.description.toLowerCase().includes(skillSearch.toLowerCase()),
      )
    : skills;

  // Handle skill selection from hint
  const insertSkill = (skill: SkillSummary) => {
    const skillCommand = `/${skill.id}`;
    setNewMessage((prev) => {
      const slashIndex = prev.lastIndexOf('/');
      if (slashIndex >= 0) {
        return prev.substring(0, slashIndex) + skillCommand + ' ';
      }
      return prev + skillCommand + ' ';
    });
    setShowSkillHint(false);
    setSkillSearch('');
    inputRef.current?.focus();
  };

  // Handle click outside skill hint
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        skillHintRef.current &&
        !skillHintRef.current.contains(e.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(e.target as Node)
      ) {
        setShowSkillHint(false);
        setSkillSearch('');
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSend = () => {
    if (!newMessage.trim()) return;
    onSend(newMessage.trim());
    setNewMessage('');
  };

  return (
    <div className="bg-card rounded-lg border p-4">
      <div className="flex gap-2 relative">
        <div className="relative flex-1">
          {atEnabled && roles.length > 0 && (
            <RoleMentionPicker
              roles={roles}
              value={newMessage}
              onChange={setNewMessage}
              onInsert={setNewMessage}
              inputRef={inputRef}
            />
          )}
          <input
            ref={inputRef}
            type="text"
            value={newMessage}
            onChange={(e) => {
              const value = e.target.value;
              setNewMessage(value);
              // Detect slash command
              const lastSlashIndex = value.lastIndexOf('/');
              if (lastSlashIndex >= 0 && lastSlashIndex === value.length - 1) {
                setShowSkillHint(true);
                setSkillSearch('');
                setSelectedSkillIndex(0);
              } else if (lastSlashIndex >= 0) {
                const afterSlash = value.substring(lastSlashIndex + 1);
                if (afterSlash.includes(' ') || afterSlash.includes('\n')) {
                  setShowSkillHint(false);
                } else {
                  setShowSkillHint(true);
                  setSkillSearch(afterSlash);
                  setSelectedSkillIndex(0);
                }
              } else {
                setShowSkillHint(false);
                setSkillSearch('');
              }
            }}
            onKeyDown={(e) => {
              if (isComposingRef.current) return;
              if (showSkillHint) {
                if (e.key === 'ArrowDown') {
                  e.preventDefault();
                  setSelectedSkillIndex((prev) =>
                    prev < filteredSkills.length - 1 ? prev + 1 : prev,
                  );
                } else if (e.key === 'ArrowUp') {
                  e.preventDefault();
                  setSelectedSkillIndex((prev) => (prev > 0 ? prev - 1 : prev));
                } else if (e.key === 'Enter' && filteredSkills.length > 0) {
                  e.preventDefault();
                  insertSkill(filteredSkills[selectedSkillIndex]);
                } else if (e.key === 'Escape') {
                  setShowSkillHint(false);
                  setSkillSearch('');
                } else if (e.key === 'Enter') {
                  handleSend();
                }
              } else if (e.key === 'Enter') {
                handleSend();
              }
            }}
            placeholder={
              atEnabled && roles.length > 0
                ? '输入消息... (输入 / 触发技能，@ 角色)'
                : '输入消息... (输入 / 触发技能)'
            }
            className="input flex-1 pr-20"
            onCompositionStart={() => {
              isComposingRef.current = true;
            }}
            onCompositionEnd={() => {
              isComposingRef.current = false;
            }}
          />
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
            <Sparkles className="w-4 h-4" />
          </span>
        </div>
        <button
          onClick={handleSend}
          disabled={!newMessage.trim() || disabled}
          className="btn btn-primary"
        >
          <Send className="w-4 h-4" />
        </button>
      </div>

      {/* Skill Hint Popup */}
      {showSkillHint && (
        <div
          ref={skillHintRef}
          className="absolute bottom-full left-4 right-4 mb-2 bg-card rounded-lg border shadow-lg max-h-64 overflow-y-auto z-50"
        >
          <div className="sticky top-0 bg-card border-b px-3 py-2">
            <p className="text-xs text-muted-foreground">
              {skillSearch ? '搜索技能...' : '可用技能'}
            </p>
          </div>
          {filteredSkills.length === 0 ? (
            <div className="px-3 py-4 text-center text-sm text-muted-foreground">未找到技能</div>
          ) : (
            <div className="py-1">
              {filteredSkills.map((skill, index) => (
                <button
                  key={skill.id}
                  onClick={() => insertSkill(skill)}
                  className={cn(
                    'w-full text-left px-3 py-2 hover:bg-accent transition-colors',
                    index === selectedSkillIndex && 'bg-accent',
                  )}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-sm">
                      <Sparkles className="w-3 h-3 inline mr-2 text-primary" />/{skill.id}
                    </span>
                    <span className="text-xs text-muted-foreground">{skill.category}</span>
                  </div>
                  <p className="text-xs text-muted-foreground mt-0.5 truncate">
                    {skill.description}
                  </p>
                </button>
              ))}
            </div>
          )}
          <div className="sticky bottom-0 bg-card border-t px-3 py-2 text-xs text-muted-foreground">
            ↑↓ 选择 • Enter 插入 • Esc 关闭
          </div>
        </div>
      )}
    </div>
  );
}
