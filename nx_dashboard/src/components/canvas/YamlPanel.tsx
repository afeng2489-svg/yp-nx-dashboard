import { useState } from 'react';
import { useCanvasStore } from '@/stores/canvasStore';

export function YamlPanel() {
  const { toYaml, loadFromYaml } = useCanvasStore();
  const [mode, setMode] = useState<'preview' | 'import'>('preview');
  const [importText, setImportText] = useState('');

  const yamlStr = toYaml();

  const download = () => {
    const blob = new Blob([yamlStr], { type: 'text/yaml' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'workflow.yaml';
    a.click();
  };

  const handleImport = () => {
    loadFromYaml(importText);
    setMode('preview');
    setImportText('');
  };

  return (
    <div className="flex h-full flex-col border-l border-border bg-card w-64 shrink-0">
      <div className="flex border-b border-border">
        <button
          onClick={() => setMode('preview')}
          className={`flex-1 py-2 text-xs transition-colors ${mode === 'preview' ? 'text-foreground bg-accent' : 'text-muted-foreground hover:text-foreground'}`}
        >
          YAML 预览
        </button>
        <button
          onClick={() => setMode('import')}
          className={`flex-1 py-2 text-xs transition-colors ${mode === 'import' ? 'text-foreground bg-accent' : 'text-muted-foreground hover:text-foreground'}`}
        >
          导入
        </button>
      </div>

      {mode === 'preview' ? (
        <>
          <div className="flex gap-1 p-2 border-b border-border">
            <button
              onClick={() => navigator.clipboard.writeText(yamlStr)}
              className="flex-1 rounded bg-secondary py-1 text-xs text-secondary-foreground hover:bg-secondary/80"
            >
              复制
            </button>
            <button
              onClick={download}
              className="flex-1 rounded bg-secondary py-1 text-xs text-secondary-foreground hover:bg-secondary/80"
            >
              下载
            </button>
          </div>
          <pre className="flex-1 overflow-auto p-3 text-xs text-green-500 dark:text-green-400 font-mono whitespace-pre-wrap">
            {yamlStr}
          </pre>
        </>
      ) : (
        <div className="flex flex-col flex-1 p-2 gap-2">
          <textarea
            className="flex-1 rounded bg-background p-2 text-xs font-mono border border-border focus:outline-none focus:border-primary resize-none"
            placeholder="粘贴 YAML..."
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
          />
          <button
            onClick={handleImport}
            className="rounded bg-primary py-1.5 text-xs text-primary-foreground hover:bg-primary/90"
          >
            导入到画布
          </button>
        </div>
      )}
    </div>
  );
}
