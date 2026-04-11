import { useState, useEffect } from 'react';
import { X, Zap, Loader2, Check, Copy, ExternalLink } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTeamStore, TelegramConfig } from '@/stores/teamStore';
import { API_BASE_URL } from '@/api/constants';

interface TelegramConfigPanelProps {
  teamId: string;
  onClose: () => void;
}

export function TelegramConfigPanel({ teamId, onClose }: TelegramConfigPanelProps) {
  const { getTelegramConfig, configureTelegram, enableTelegram, telegramConfig } = useTeamStore();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [botToken, setBotToken] = useState('');
  const [chatId, setChatId] = useState('');
  const [enabled, setEnabled] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    loadConfig();
  }, [teamId]);

  const loadConfig = async () => {
    setLoading(true);
    try {
      const config = await getTelegramConfig(teamId);
      if (config) {
        setBotToken(config.bot_token || '');
        setChatId(config.chat_id || '');
        setEnabled(config.enabled);
      }
    } catch (error) {
      console.error('Failed to load Telegram config:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await configureTelegram(teamId, {
        bot_token: botToken.trim() || undefined,
        chat_id: chatId.trim() || undefined,
        enabled,
      });
    } catch (error) {
      console.error('Failed to save Telegram config:', error);
    } finally {
      setSaving(false);
    }
  };

  const handleToggleEnabled = async () => {
    const newEnabled = !enabled;
    setEnabled(newEnabled);
    try {
      await enableTelegram(teamId, newEnabled);
    } catch (error) {
      setEnabled(!newEnabled);
      console.error('Failed to toggle Telegram:', error);
    }
  };

  const handleCopyWebhookUrl = () => {
    const url = `${API_BASE_URL}/api/v1/teams/${teamId}/telegram/webhook`;
    navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (loading) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
        <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 p-8">
          <Loader2 className="w-8 h-8 animate-spin text-muted-foreground mx-auto" />
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md bg-card rounded-2xl shadow-2xl border border-border/50 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/50 bg-gradient-to-r from-blue-500/5 to-cyan-500/5">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-gradient-to-br from-blue-500 to-cyan-500 shadow-lg shadow-blue-500/25">
              <Zap className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">Telegram 配置</h2>
              <p className="text-xs text-muted-foreground">接收团队通知</p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-accent transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          {/* Enable Toggle */}
          <div className="flex items-center justify-between p-4 bg-gradient-to-r from-blue-500/5 to-cyan-500/5 rounded-xl border border-blue-500/10">
            <div className="flex items-center gap-3">
              <div className={cn(
                'w-10 h-10 rounded-full flex items-center justify-center',
                enabled ? 'bg-blue-500' : 'bg-muted'
              )}>
                <Zap className={cn('w-5 h-5', enabled ? 'text-white' : 'text-muted-foreground')} />
              </div>
              <div>
                <p className="font-medium">启用 Telegram</p>
                <p className="text-xs text-muted-foreground">
                  {enabled ? '已启用通知推送' : '未启用'}
                </p>
              </div>
            </div>
            <button
              onClick={handleToggleEnabled}
              className={cn(
                'relative w-12 h-6 rounded-full transition-colors',
                enabled ? 'bg-blue-500' : 'bg-muted'
              )}
            >
              <div className={cn(
                'absolute top-1 w-4 h-4 rounded-full bg-white transition-transform',
                enabled ? 'translate-x-7' : 'translate-x-1'
              )} />
            </button>
          </div>

          {/* Bot Token */}
          <div>
            <label className="block text-sm font-medium mb-2">Bot Token</label>
            <input
              type="password"
              value={botToken}
              onChange={(e) => setBotToken(e.target.value)}
              placeholder="从 @BotFather 获取的 token"
              className="input-field"
            />
            <p className="text-xs text-muted-foreground mt-1">
              格式: 123456789:ABCdefGHIjklMNOpqrSTUvwxyz
            </p>
          </div>

          {/* Chat ID */}
          <div>
            <label className="block text-sm font-medium mb-2">Chat ID</label>
            <input
              type="text"
              value={chatId}
              onChange={(e) => setChatId(e.target.value)}
              placeholder="您的 Telegram Chat ID"
              className="input-field"
            />
            <p className="text-xs text-muted-foreground mt-1">
              发送消息给 @userinfobot 获取您的 Chat ID
            </p>
          </div>

          {/* Webhook URL */}
          <div>
            <label className="block text-sm font-medium mb-2">Webhook URL</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={`${API_BASE_URL}/api/v1/teams/${teamId}/telegram/webhook`}
                readOnly
                className="input-field flex-1 text-xs"
              />
              <button
                onClick={handleCopyWebhookUrl}
                className="btn-secondary px-3"
                title="复制"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-500" />
                ) : (
                  <Copy className="w-4 h-4" />
                )}
              </button>
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              在 Telegram Bot 设置中配置此 webhook URL
            </p>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-border/50">
            <button onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button onClick={handleSave} disabled={saving} className="btn-primary">
              {saving ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  保存中...
                </>
              ) : (
                <>保存</>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
