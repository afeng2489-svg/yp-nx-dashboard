import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowRight, BookOpen, Layers, Sparkles, ChevronRight } from 'lucide-react';
import { scenarios } from '@/data/guide/scenarios';
import { modules, type ModuleInfo } from '@/data/guide/modules';
import type { ScenarioStep } from '@/data/guide/types';

export function GuidePage() {
  const navigate = useNavigate();
  const [activeId, setActiveId] = useState<string>(scenarios[0]?.id ?? '');
  const active = scenarios.find((s) => s.id === activeId) ?? scenarios[0];

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-6xl mx-auto px-6 py-8">
        {/* Hero */}
        <div className="mb-8">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-primary/10 text-primary text-xs mb-3">
            <Sparkles className="w-3.5 h-3.5" />
            使用指南
          </div>
          <h1 className="text-3xl font-bold text-foreground mb-2">挑一个开发场景，跟着走一遍</h1>
          <p className="text-muted-foreground">
            每个场景按推荐顺序列出用到的模块，点击步骤可直达对应页面。
          </p>
        </div>

        {/* 场景 tabs */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 mb-8">
          {scenarios.map((s) => {
            const isActive = s.id === activeId;
            return (
              <button
                key={s.id}
                onClick={() => setActiveId(s.id)}
                className={`p-4 rounded-2xl border transition-all text-left ${
                  isActive
                    ? 'bg-gradient-to-br from-indigo-500/10 via-purple-500/10 to-pink-500/10 border-primary/40 shadow-md scale-[1.02]'
                    : 'bg-card border-border hover:border-primary/30 hover:shadow-sm'
                }`}
              >
                <div className="text-2xl mb-2">{s.emoji}</div>
                <div
                  className={`text-sm font-semibold mb-1 ${
                    isActive ? 'text-primary' : 'text-foreground'
                  }`}
                >
                  {s.name}
                </div>
                <div className="text-xs text-muted-foreground line-clamp-2">{s.description}</div>
              </button>
            );
          })}
        </div>

        {/* 活动场景 */}
        {active && (
          <div className="bg-card border border-border rounded-2xl shadow-sm overflow-hidden">
            <div className="px-6 py-4 border-b border-border bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5">
              <div className="flex items-center gap-3 mb-1">
                <span className="text-3xl">{active.emoji}</span>
                <h2 className="text-xl font-semibold text-foreground">{active.name}</h2>
              </div>
              <p className="text-sm text-muted-foreground">{active.description}</p>
              {active.highlight && (
                <div className="mt-3 flex items-start gap-2 p-3 rounded-lg bg-amber-500/5 border border-amber-500/20">
                  <Sparkles className="w-4 h-4 text-amber-500 mt-0.5 shrink-0" />
                  <div className="text-xs text-amber-700 dark:text-amber-400 leading-relaxed">
                    {active.highlight}
                  </div>
                </div>
              )}
            </div>

            <div className="p-6">
              <div className="text-sm font-semibold text-foreground mb-4 flex items-center gap-2">
                <Layers className="w-4 h-4 text-primary" />
                推荐使用顺序（共 {active.steps.length} 步）
              </div>
              <ol className="space-y-2">
                {active.steps.map((step: ScenarioStep, i: number) => {
                  const mod = modules[step.moduleId];
                  const isLast = i === active.steps.length - 1;
                  return (
                    <li key={i} className="relative">
                      <button
                        onClick={() => mod && navigate(mod.path)}
                        disabled={!mod}
                        className={`w-full flex items-start gap-4 p-3 rounded-xl border transition-all group ${
                          mod
                            ? 'border-border hover:border-primary/40 hover:bg-accent/50 cursor-pointer'
                            : 'border-border opacity-50 cursor-not-allowed'
                        }`}
                      >
                        <div className="w-8 h-8 rounded-lg bg-primary/10 text-primary flex items-center justify-center text-xs font-semibold shrink-0">
                          {i + 1}
                        </div>
                        <div className="flex-1 text-left">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="font-medium text-foreground">{step.action}</span>
                            {mod && (
                              <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded bg-accent text-muted-foreground">
                                {mod.label}
                              </span>
                            )}
                          </div>
                          {step.detail && (
                            <div className="text-xs text-muted-foreground mt-1 leading-relaxed">
                              {step.detail}
                            </div>
                          )}
                        </div>
                        {mod && (
                          <ArrowRight className="w-4 h-4 text-muted-foreground group-hover:text-primary group-hover:translate-x-0.5 transition-all mt-2" />
                        )}
                      </button>
                      {!isLast && (
                        <div className="absolute left-7 top-[calc(100%-2px)] w-0.5 h-2 bg-border" />
                      )}
                    </li>
                  );
                })}
              </ol>
            </div>
          </div>
        )}

        {/* 模块总览 */}
        <div className="mt-10">
          <div className="flex items-center gap-2 mb-4">
            <BookOpen className="w-5 h-5 text-primary" />
            <h2 className="text-xl font-semibold text-foreground">所有模块</h2>
          </div>
          <ModuleIndex />
        </div>
      </div>
    </div>
  );
}

function ModuleIndex() {
  const navigate = useNavigate();
  const grouped = Object.values(modules).reduce<Record<string, ModuleInfo[]>>((acc, m) => {
    (acc[m.group] ||= []).push(m);
    return acc;
  }, {});

  return (
    <div className="space-y-5">
      {Object.entries(grouped).map(([group, list]) => (
        <div key={group}>
          <div className="text-xs uppercase tracking-wider text-muted-foreground mb-2 font-medium">
            {group}
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
            {list.map((m: ModuleInfo) => (
              <button
                key={m.id}
                onClick={() => navigate(m.path)}
                className="flex items-start justify-between gap-3 p-3 rounded-xl border border-border bg-card hover:border-primary/40 hover:shadow-sm transition-all text-left group"
              >
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-foreground text-sm mb-0.5">{m.label}</div>
                  <div className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
                    {m.purpose}
                  </div>
                </div>
                <ChevronRight className="w-4 h-4 text-muted-foreground group-hover:text-primary shrink-0 mt-0.5" />
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
