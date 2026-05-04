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
    <div className="flex h-full flex-col border-l border-zinc-800 bg-zinc-950 w-64 shrink-0">
      <div className="flex border-b border-zinc-800">
        <button
          onClick={() => setMode('preview')}
          className={`flex-1 py-2 text-xs ${mode === 'preview' ? 'text-white bg-zinc-800' : 'text-zinc-500 hover:text-zinc-300'}`}
        >
          YAML 预览
        </button>
        <button
          onClick={() => setMode('import')}
          className={`flex-1 py-2 text-xs ${mode === 'import' ? 'text-white bg-zinc-800' : 'text-zinc-500 hover:text-zinc-300'}`}
        >
          导入
        </button>
      </div>

      {mode === 'preview' ? (
        <>
          <div className="flex gap-1 p-2 border-b border-zinc-800">
            <button
              onClick={() => navigator.clipboard.writeText(yamlStr)}
              className="flex-1 rounded bg-zinc-800 py-1 text-xs text-zinc-300 hover:bg-zinc-700"
            >
              复制
            </button>
            <button
              onClick={download}
              className="flex-1 rounded bg-zinc-800 py-1 text-xs text-zinc-300 hover:bg-zinc-700"
            >
              下载
            </button>
          </div>
          <pre className="flex-1 overflow-auto p-3 text-xs text-green-400 font-mono whitespace-pre-wrap">
            {yamlStr}
          </pre>
        </>
      ) : (
        <div className="flex flex-col flex-1 p-2 gap-2">
          <textarea
            className="flex-1 rounded bg-zinc-800 p-2 text-xs text-zinc-200 font-mono border border-zinc-700 focus:outline-none resize-none"
            placeholder="粘贴 YAML..."
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
          />
          <button
            onClick={handleImport}
            className="rounded bg-blue-600 py-1.5 text-xs text-white hover:bg-blue-500"
          >
            导入到画布
          </button>
        </div>
      )}
    </div>
  );
}
