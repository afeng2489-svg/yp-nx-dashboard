import { X, HelpCircle, ArrowRight } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { modules, type ModuleInfo } from '@/data/guide/modules';

interface ModuleHelpModalProps {
  moduleId: string;
  onClose: () => void;
}

export function ModuleHelpModal({ moduleId, onClose }: ModuleHelpModalProps) {
  const info: ModuleInfo | undefined = modules[moduleId];
  const navigate = useNavigate();

  if (!info) {
    return (
      <div
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
        onClick={onClose}
      >
        <div className="bg-card rounded-2xl p-6 max-w-md" onClick={(e) => e.stopPropagation()}>
          <div className="text-sm text-muted-foreground">暂无此模块的说明</div>
        </div>
      </div>
    );
  }

  const related = info.relatedTo
    .map((id: string) => modules[id])
    .filter((m: ModuleInfo | undefined): m is ModuleInfo => Boolean(m));

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4"
      onClick={onClose}
    >
      <div
        className="bg-card rounded-2xl shadow-2xl border border-border w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* header */}
        <div className="flex items-start justify-between px-6 py-4 border-b border-border bg-gradient-to-r from-indigo-500/5 via-purple-500/5 to-pink-500/5">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
              <HelpCircle className="w-5 h-5 text-primary" />
            </div>
            <div>
              <div className="text-xs text-muted-foreground">{info.group}</div>
              <h2 className="text-xl font-semibold text-foreground">{info.label}</h2>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-accent rounded-lg text-muted-foreground"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* body */}
        <div className="flex-1 overflow-y-auto px-6 py-5 space-y-5">
          <section>
            <h3 className="text-sm font-semibold text-foreground mb-2">这个模块是做什么的</h3>
            <p className="text-sm text-muted-foreground leading-relaxed">{info.purpose}</p>
          </section>

          <section>
            <h3 className="text-sm font-semibold text-foreground mb-2">什么时候用</h3>
            <ul className="space-y-1.5">
              {info.when.map((w: string, i: number) => (
                <li key={i} className="text-sm text-muted-foreground flex items-start gap-2">
                  <span className="text-primary mt-1">•</span>
                  <span className="flex-1">{w}</span>
                </li>
              ))}
            </ul>
          </section>

          {info.tips && info.tips.length > 0 && (
            <section>
              <h3 className="text-sm font-semibold text-foreground mb-2">使用要点</h3>
              <ul className="space-y-1.5">
                {info.tips.map((t: string, i: number) => (
                  <li key={i} className="text-sm text-muted-foreground flex items-start gap-2">
                    <span className="text-amber-500 mt-1">✓</span>
                    <span className="flex-1">{t}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {related.length > 0 && (
            <section>
              <h3 className="text-sm font-semibold text-foreground mb-2">相关模块</h3>
              <div className="flex flex-wrap gap-2">
                {related.map((r: ModuleInfo) => (
                  <button
                    key={r.id}
                    onClick={() => {
                      onClose();
                      navigate(r.path);
                    }}
                    className="inline-flex items-center gap-1 px-3 py-1.5 text-xs rounded-lg bg-accent hover:bg-accent/80 text-foreground transition-colors"
                  >
                    {r.label}
                    <ArrowRight className="w-3 h-3" />
                  </button>
                ))}
              </div>
            </section>
          )}
        </div>

        {/* footer */}
        <div className="px-6 py-3 border-t border-border flex items-center justify-between gap-3 bg-accent/30">
          <span className="text-xs text-muted-foreground">想看完整使用场景？打开「使用指南」</span>
          <button
            onClick={() => {
              onClose();
              navigate('/guide');
            }}
            className="px-4 py-1.5 text-sm rounded-lg btn-primary"
          >
            去使用指南
          </button>
        </div>
      </div>
    </div>
  );
}
