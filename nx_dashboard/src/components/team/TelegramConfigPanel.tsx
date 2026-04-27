import { useState, useEffect } from 'react';
import {
  X,
  Loader2,
  Check,
  Bot,
  Power,
  PowerOff,
  Save,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, MemberBotStatus } from '@/stores/teamStore';

interface TelegramConfigPanelProps {
  teamId: string;
  onClose: () => void;
}

interface MemberRowState {
  botToken: string;
  chatId: string;
  saving: boolean;
  saved: boolean;
  expanded: boolean;
  fetching: boolean;
  fetchError: string | null;
  botUsername: string | null;
}

export function TelegramConfigPanel({ teamId, onClose }: TelegramConfigPanelProps) {
  const { getMemberBots, configureMemberBot, toggleAllMemberBots } = useTeamStore();
  const [loading, setLoading] = useState(true);
  const [members, setMembers] = useState<MemberBotStatus[]>([]);
  const [rowStates, setRowStates] = useState<Record<string, MemberRowState>>({});
  const [togglingAll, setTogglingAll] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    loadBots();
  }, [teamId]);

  const loadBots = async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await getMemberBots(teamId);
      setMembers(data);
      const initial: Record<string, MemberRowState> = {};
      for (const m of data) {
        initial[m.role_id] = {
          botToken: m.bot_config?.bot_token ?? '',
          chatId: m.bot_config?.chat_id ?? '',
          saving: false,
          saved: false,
          expanded: false,
          fetching: false,
          fetchError: null,
          botUsername: null,
        };
      }
      setRowStates(initial);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setLoadError(msg);
    } finally {
      setLoading(false);
    }
  };

  const updateRow = (roleId: string, patch: Partial<MemberRowState>) => {
    setRowStates((prev) => ({ ...prev, [roleId]: { ...prev[roleId], ...patch } }));
  };

  const autoFetchChatId = async (roleId: string) => {
    const token = rowStates[roleId]?.botToken.trim();
    if (!token) return;
    updateRow(roleId, { fetching: true, fetchError: null, botUsername: null, chatId: '' });
    try {
      // 1. Verify token and get bot username
      const meRes = await fetch(`https://api.telegram.org/bot${token}/getMe`);
      const meData = await meRes.json();
      if (!meData.ok) {
        updateRow(roleId, { fetchError: 'Token 无效，请检查后重试' });
        return;
      }
      const username = meData.result?.username ?? meData.result?.first_name ?? 'Bot';
      updateRow(roleId, { botUsername: username });

      // 2. Get latest chat_id from updates
      const updRes = await fetch(`https://api.telegram.org/bot${token}/getUpdates?limit=10`);
      const updData = await updRes.json();
      if (updData.ok && updData.result?.length > 0) {
        const lastUpdate = updData.result[updData.result.length - 1];
        const chatId =
          lastUpdate.message?.chat?.id ?? lastUpdate.callback_query?.message?.chat?.id ?? null;
        if (chatId !== null) {
          updateRow(roleId, { chatId: String(chatId), fetchError: null });
        } else {
          updateRow(roleId, { fetchError: '未找到 chat_id，请先给 Bot 发一条消息后重试' });
        }
      } else {
        updateRow(roleId, { fetchError: '暂无消息记录，请先给 Bot 发一条消息后重试' });
      }
    } catch {
      updateRow(roleId, { fetchError: '网络错误，请检查 Token 后重试' });
    } finally {
      updateRow(roleId, { fetching: false });
    }
  };

  const handleSaveMember = async (roleId: string) => {
    const row = rowStates[roleId];
    if (!row?.botToken.trim()) return;
    updateRow(roleId, { saving: true });
    try {
      const updated = await configureMemberBot(teamId, roleId, {
        role_id: roleId,
        bot_token: row.botToken.trim(),
        chat_id: row.chatId.trim() || undefined,
      });
      setMembers((prev) => prev.map((m) => (m.role_id === roleId ? updated : m)));
      updateRow(roleId, { saved: true });
      setTimeout(() => updateRow(roleId, { saved: false }), 2000);
    } catch (e) {
      console.error('Failed to save bot config', e);
    } finally {
      updateRow(roleId, { saving: false });
    }
  };

  const handleToggleAll = async (enabled: boolean) => {
    setTogglingAll(true);
    try {
      const updated = await toggleAllMemberBots(teamId, enabled);
      setMembers(updated);
    } catch (e) {
      console.error('Failed to toggle all bots', e);
    } finally {
      setTogglingAll(false);
    }
  };

  const anyPolling = members.some((m) => m.is_polling);
  const configuredCount = members.filter((m) => m.bot_config?.bot_token).length;

  if (loading) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
        <div className="relative w-full max-w-2xl bg-card rounded-2xl shadow-2xl border border-border/50 p-8 flex justify-center">
          <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-2xl bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-blue-500/5 to-cyan-500/5 flex-shrink-0">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-500 shadow-lg shadow-blue-500/25">
              <Bot className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">成员 Bot 分配</h2>
              <p className="text-xs text-muted-foreground">
                为团队每个成员单独绑定 Telegram Bot
                {configuredCount > 0 && ` · 已配置 ${configuredCount}/${members.length}`}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {/* Toggle All */}
            <button
              onClick={() => handleToggleAll(!anyPolling)}
              disabled={togglingAll || configuredCount === 0}
              className={cn(
                'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all',
                anyPolling
                  ? 'bg-red-500/10 text-red-500 hover:bg-red-500/20'
                  : 'bg-green-500/10 text-green-600 hover:bg-green-500/20',
                (togglingAll || configuredCount === 0) && 'opacity-50 cursor-not-allowed',
              )}
            >
              {togglingAll ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : anyPolling ? (
                <PowerOff className="w-3.5 h-3.5" />
              ) : (
                <Power className="w-3.5 h-3.5" />
              )}
              {anyPolling ? '全部停止' : '全部启动'}
            </button>
            <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Members List */}
        <div className="overflow-y-auto flex-1 p-4 space-y-2">
          {members.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Bot className="w-10 h-10 mx-auto mb-3 opacity-30" />
              {loadError ? (
                <p className="text-red-500 text-sm">{loadError}</p>
              ) : (
                <p>该团队暂无成员</p>
              )}
            </div>
          ) : (
            members.map((member) => {
              const row = rowStates[member.role_id];
              if (!row) return null;
              const hasToken = !!member.bot_config?.bot_token;

              return (
                <div
                  key={member.role_id}
                  className={cn(
                    'rounded-xl border transition-all',
                    member.is_polling
                      ? 'border-green-500/30 bg-green-500/5'
                      : hasToken
                        ? 'border-blue-500/20 bg-blue-500/5'
                        : 'border-border/50 bg-card',
                  )}
                >
                  {/* Row Header */}
                  <div
                    className="flex items-center gap-3 px-4 py-3 cursor-pointer"
                    onClick={() => updateRow(member.role_id, { expanded: !row.expanded })}
                  >
                    {/* Status dot */}
                    <div
                      className={cn(
                        'w-2.5 h-2.5 rounded-full flex-shrink-0',
                        member.is_polling
                          ? 'bg-green-500 animate-pulse'
                          : hasToken
                            ? 'bg-blue-400'
                            : 'bg-muted-foreground/30',
                      )}
                    />
                    <span className="font-medium flex-1">{member.role_name}</span>
                    <span
                      className={cn(
                        'text-xs px-2 py-0.5 rounded-full',
                        member.is_polling
                          ? 'bg-green-500/15 text-green-600'
                          : hasToken
                            ? 'bg-blue-500/15 text-blue-600'
                            : 'bg-muted text-muted-foreground',
                      )}
                    >
                      {member.is_polling ? '运行中' : hasToken ? '已配置' : '未配置'}
                    </span>
                    {row.expanded ? (
                      <ChevronUp className="w-4 h-4 text-muted-foreground" />
                    ) : (
                      <ChevronDown className="w-4 h-4 text-muted-foreground" />
                    )}
                  </div>

                  {/* Expanded Config */}
                  {row.expanded && (
                    <div className="px-4 pb-4 space-y-3 border-t border-border/30 pt-3">
                      <div>
                        <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                          Bot Token
                        </label>
                        <div className="flex gap-2">
                          <input
                            type="password"
                            value={row.botToken}
                            onChange={(e) =>
                              updateRow(member.role_id, {
                                botToken: e.target.value,
                                botUsername: null,
                                chatId: '',
                                fetchError: null,
                              })
                            }
                            onBlur={() => row.botToken.trim() && autoFetchChatId(member.role_id)}
                            placeholder="从 @BotFather 获取"
                            className="input-field text-sm flex-1"
                          />
                          <button
                            onClick={() => autoFetchChatId(member.role_id)}
                            disabled={!row.botToken.trim() || row.fetching}
                            className="px-3 py-1.5 text-xs rounded-lg border bg-background hover:bg-accent transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
                            title="自动获取 Chat ID"
                          >
                            {row.fetching ? <Loader2 className="w-3 h-3 animate-spin" /> : '获取'}
                          </button>
                        </div>
                      </div>

                      {/* Auto-fetched info */}
                      {row.botUsername && (
                        <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-blue-500/10 border border-blue-500/20">
                          <Bot className="w-3.5 h-3.5 text-blue-500 flex-shrink-0" />
                          <span className="text-xs text-blue-600 font-medium">
                            @{row.botUsername}
                          </span>
                          {row.chatId && (
                            <span className="text-xs text-muted-foreground ml-auto">
                              Chat ID: {row.chatId}
                            </span>
                          )}
                        </div>
                      )}

                      {row.fetchError && (
                        <div className="px-3 py-2 rounded-lg bg-red-500/10 border border-red-500/20">
                          <p className="text-xs text-red-500">{row.fetchError}</p>
                          {row.fetchError.includes('发一条消息') && (
                            <p className="text-xs text-muted-foreground mt-1">
                              给 Bot 发消息后，点击"获取"按钮重试
                            </p>
                          )}
                        </div>
                      )}

                      <div className="flex justify-end">
                        <button
                          onClick={() => handleSaveMember(member.role_id)}
                          disabled={row.saving || !row.botToken.trim()}
                          className={cn(
                            'flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all',
                            row.saved ? 'bg-green-500/15 text-green-600' : 'btn-primary',
                            (row.saving || !row.botToken.trim()) && 'opacity-50 cursor-not-allowed',
                          )}
                        >
                          {row.saving ? (
                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                          ) : row.saved ? (
                            <Check className="w-3.5 h-3.5" />
                          ) : (
                            <Save className="w-3.5 h-3.5" />
                          )}
                          {row.saved ? '已保存' : '保存'}
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>

        {/* Footer hint */}
        <div className="px-6 py-3 border-t border-border/50 bg-muted/30 flex-shrink-0">
          <p className="text-xs text-muted-foreground">
            粘贴 Bot Token 后自动获取 Chat ID · 保存后可通过"全部启动"开启轮询接收消息
          </p>
        </div>
      </div>
    </div>
  );
}
