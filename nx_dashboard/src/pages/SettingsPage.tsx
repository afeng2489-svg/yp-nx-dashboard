import { useState } from 'react';
import { useThemeStore, Theme } from '@/stores/themeStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useKeyboardStore } from '@/lib/keyboard';
import {
  Sun,
  Moon,
  Monitor,
  Keyboard,
  Layout,
  Bell,
  Shield,
  Palette,
  Save,
  RotateCcw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { showSuccess } from '@/lib/toast';

interface SettingsSection {
  id: string;
  title: string;
  icon: React.ComponentType<{ className?: string }>;
}

const sections: SettingsSection[] = [
  { id: 'appearance', title: '外观', icon: Palette },
  { id: 'shortcuts', title: '快捷键', icon: Keyboard },
  { id: 'layout', title: '布局', icon: Layout },
  { id: 'notifications', title: '通知', icon: Bell },
  { id: 'security', title: '安全', icon: Shield },
];

// Theme selector
function ThemeSelector() {
  const { theme, setTheme } = useThemeStore();

  const themes: { value: Theme; label: string; icon: React.ComponentType<{ className?: string }>; description: string }[] = [
    { value: 'light', label: '浅色', icon: Sun, description: '明亮的浅色主题' },
    { value: 'dark', label: '深色', icon: Moon, description: '舒适的深色主题' },
    { value: 'system', label: '系统', icon: Monitor, description: '跟随系统设置' },
  ];

  return (
    <div className="space-y-3">
      <p className="text-sm font-medium">主题</p>
      <div className="grid grid-cols-3 gap-3">
        {themes.map((t) => (
          <button
            key={t.value}
            onClick={() => setTheme(t.value)}
            className={cn(
              'flex flex-col items-center gap-2 p-4 rounded-xl border transition-all duration-200',
              theme === t.value
                ? 'border-primary bg-primary/5 shadow-sm'
                : 'border-border hover:border-primary/50'
            )}
          >
            <t.icon className={cn('w-6 h-6', theme === t.value ? 'text-primary' : 'text-muted-foreground')} />
            <span className={cn('text-sm font-medium', theme === t.value && 'text-primary')}>{t.label}</span>
            <span className="text-xs text-muted-foreground">{t.description}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

// Layout settings
function LayoutSettings() {
  const { layout, setLayout } = useSettingsStore();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">默认展开侧边栏</p>
          <p className="text-xs text-muted-foreground">页面加载时自动展开侧边栏</p>
        </div>
        <ToggleSwitch
          checked={layout.sidebarOpen}
          onChange={(v) => setLayout({ sidebarOpen: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">紧凑模式</p>
          <p className="text-xs text-muted-foreground">使用更小的间距和字体</p>
        </div>
        <ToggleSwitch
          checked={layout.compactMode}
          onChange={(v) => setLayout({ compactMode: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">动画效果</p>
          <p className="text-xs text-muted-foreground">启用页面过渡和微交互</p>
        </div>
        <ToggleSwitch
          checked={layout.animations}
          onChange={(v) => setLayout({ animations: v })}
        />
      </div>
    </div>
  );
}

// Notification settings
function NotificationSettings() {
  const { notifications, setNotifications } = useSettingsStore();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">执行完成通知</p>
          <p className="text-xs text-muted-foreground">工作流执行完成时发送通知</p>
        </div>
        <ToggleSwitch
          checked={notifications.executionComplete}
          onChange={(v) => setNotifications({ executionComplete: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">执行失败通知</p>
          <p className="text-xs text-muted-foreground">工作流执行失败时发送通知</p>
        </div>
        <ToggleSwitch
          checked={notifications.executionFailed}
          onChange={(v) => setNotifications({ executionFailed: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">会话更新</p>
          <p className="text-xs text-muted-foreground">会话状态变化时发送通知</p>
        </div>
        <ToggleSwitch
          checked={notifications.sessionUpdate}
          onChange={(v) => setNotifications({ sessionUpdate: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">周报</p>
          <p className="text-xs text-muted-foreground">每周发送工作流执行统计</p>
        </div>
        <ToggleSwitch
          checked={notifications.weeklyReport}
          onChange={(v) => setNotifications({ weeklyReport: v })}
        />
      </div>
    </div>
  );
}

// Keyboard shortcuts settings
function ShortcutsSettings() {
  const { isEnabled, setEnabled } = useKeyboardStore();

  const shortcutList = [
    { id: 'toggle-sidebar', label: '切换侧边栏', shortcut: '⌘B' },
    { id: 'toggle-theme', label: '切换主题', shortcut: '⌘D' },
    { id: 'command-palette', label: '命令面板', shortcut: '⌘K' },
    { id: 'go-dashboard', label: '前往仪表盘', shortcut: 'G D' },
    { id: 'go-workflows', label: '前往工作流', shortcut: 'G W' },
    { id: 'go-executions', label: '前往执行', shortcut: 'G E' },
    { id: 'save', label: '保存', shortcut: '⌘S' },
  ];

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">启用快捷键</p>
          <p className="text-xs text-muted-foreground">全局键盘快捷键</p>
        </div>
        <ToggleSwitch checked={isEnabled} onChange={(v) => setEnabled(v)} />
      </div>

      <div className="rounded-xl border border-border overflow-hidden">
        {shortcutList.map((item, idx) => (
          <div
            key={item.id}
            className={cn(
              'flex items-center justify-between px-4 py-3',
              idx !== shortcutList.length - 1 && 'border-b border-border'
            )}
          >
            <span className="text-sm">{item.label}</span>
            <kbd className="px-2 py-1 text-xs font-mono bg-muted rounded border border-border">
              {item.shortcut}
            </kbd>
          </div>
        ))}
      </div>
    </div>
  );
}

// Security settings
function SecuritySettings() {
  const { security, setSecurity } = useSettingsStore();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">沙箱执行</p>
          <p className="text-xs text-muted-foreground">在隔离环境中执行代码</p>
        </div>
        <ToggleSwitch
          checked={security.sandboxExecution}
          onChange={(v) => setSecurity({ sandboxExecution: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">资源限制</p>
          <p className="text-xs text-muted-foreground">限制 CPU、内存和执行时间</p>
        </div>
        <ToggleSwitch
          checked={security.resourceLimits}
          onChange={(v) => setSecurity({ resourceLimits: v })}
        />
      </div>

      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium">代码审查</p>
          <p className="text-xs text-muted-foreground">执行前自动审查代码安全性</p>
        </div>
        <ToggleSwitch
          checked={security.codeReview}
          onChange={(v) => setSecurity({ codeReview: v })}
        />
      </div>
    </div>
  );
}

// Toggle switch component
function ToggleSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={cn(
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
        checked ? 'bg-primary' : 'bg-muted'
      )}
    >
      <span
        className={cn(
          'inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform',
          checked ? 'translate-x-6' : 'translate-x-1'
        )}
      />
    </button>
  );
}

// Import useUIStore
import { useUIStore } from '@/stores/uiStore';

export function SettingsPage() {
  const [activeSection, setActiveSection] = useState('appearance');
  const { reset } = useSettingsStore();

  const renderContent = () => {
    switch (activeSection) {
      case 'appearance':
        return <ThemeSelector />;
      case 'shortcuts':
        return <ShortcutsSettings />;
      case 'layout':
        return <LayoutSettings />;
      case 'notifications':
        return <NotificationSettings />;
      case 'security':
        return <SecuritySettings />;
      default:
        return <ThemeSelector />;
    }
  };

  return (
    <div className="page-container max-w-4xl mx-auto">
      <div className="mb-6">
        <h1 className="text-3xl font-bold tracking-tight">
          <span className="bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent">
            设置
          </span>
        </h1>
        <p className="text-muted-foreground mt-1">自定义您的偏好设置</p>
      </div>

      <div className="flex gap-6">
        {/* Sidebar */}
        <div className="w-48 flex-shrink-0">
          <nav className="space-y-1">
            {sections.map((section) => (
              <button
                key={section.id}
                onClick={() => setActiveSection(section.id)}
                className={cn(
                  'w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors',
                  activeSection === section.id
                    ? 'bg-primary/10 text-primary font-medium'
                    : 'text-muted-foreground hover:bg-accent hover:text-foreground'
                )}
              >
                <section.icon className="w-4 h-4" />
                {section.title}
              </button>
            ))}
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1 bg-card rounded-2xl border border-border/50 p-6">
          {renderContent()}

          {/* Actions */}
          <div className="flex items-center gap-3 mt-8 pt-6 border-t border-border">
            <button
              onClick={() => showSuccess('设置已保存')}
              className="btn-primary"
            >
              <Save className="w-4 h-4" />
              保存设置
            </button>
            <button
              onClick={() => {
                reset();
                showSuccess('已恢复默认设置');
              }}
              className="btn-secondary"
            >
              <RotateCcw className="w-4 h-4" />
              恢复默认
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
