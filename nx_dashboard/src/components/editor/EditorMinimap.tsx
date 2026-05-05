import { useCallback, useRef, useEffect } from 'react';
import { create } from 'zustand';
import { MiniMap as ReactFlowMinimap, Panel } from '@xyflow/react';
import { useEditorStore, NodeData } from '@/stores/editorStore';
import { ZoomIn, ZoomOut, Maximize2, Undo2, Redo2 } from 'lucide-react';

interface EditorToolbarProps {
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onFitView?: () => void;
}

export function EditorToolbar({ onZoomIn, onZoomOut, onFitView }: EditorToolbarProps) {
  return (
    <div className="absolute top-4 right-4 z-10 flex items-center gap-2 p-2 bg-card/80 backdrop-blur-sm rounded-xl border border-border/50 shadow-lg">
      <ToolbarButton icon={Undo2} title="撤销 (Ctrl+Z)" />
      <ToolbarButton icon={Redo2} title="重做 (Ctrl+Y)" />
      <div className="w-px h-6 bg-border mx-1" />
      <ToolbarButton icon={ZoomOut} title="缩小" onClick={onZoomOut} />
      <ToolbarButton icon={ZoomIn} title="放大" onClick={onZoomIn} />
      <ToolbarButton icon={Maximize2} title="适应屏幕" onClick={onFitView} />
    </div>
  );
}

function ToolbarButton({
  icon: Icon,
  title,
  onClick,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className="p-1.5 rounded-lg hover:bg-accent transition-colors text-muted-foreground hover:text-foreground"
    >
      <Icon className="w-4 h-4" />
    </button>
  );
}

// Minimap component
export function EditorMinimap() {
  const nodeColor = useCallback((node: { type?: string; data?: { type?: string } }) => {
    switch (node.data?.type) {
      case 'agent':
        return '#6366f1';
      case 'stage':
        return '#8b5cf6';
      case 'condition':
        return '#f59e0b';
      case 'loop':
        return '#10b981';
      default:
        return '#94a3b8';
    }
  }, []);

  return (
    <Panel position="bottom-left" className="z-10">
      <div className="bg-card/80 backdrop-blur-sm rounded-xl border border-border/50 shadow-lg overflow-hidden">
        <ReactFlowMinimap
          nodeColor={nodeColor}
          nodeStrokeWidth={2}
          zoomable
          pannable
          style={{
            width: 150,
            height: 100,
          }}
          maskColor="rgba(0, 0, 0, 0.1)"
        />
      </div>
    </Panel>
  );
}

// History state type
interface HistoryState {
  nodes: { id: string; position: { x: number; y: number }; data: NodeData }[];
  edges: { id: string; source: string; target: string }[];
}

// Editor history store
interface EditorHistoryStore {
  past: HistoryState[];
  future: HistoryState[];
  canUndo: boolean;
  canRedo: boolean;
  pushState: (state: HistoryState) => void;
  undo: () => HistoryState | null;
  redo: () => HistoryState | null;
  clear: () => void;
}

const useEditorHistoryStore = create<EditorHistoryStore>((set, get) => ({
  past: [],
  future: [],
  canUndo: false,
  canRedo: false,

  pushState: (state) => {
    set((s) => ({
      past: [...s.past.slice(-50), state],
      future: [],
      canUndo: true,
      canRedo: false,
    }));
  },

  undo: () => {
    const { past, future } = get();
    if (past.length === 0) return null;

    const previous = past[past.length - 1];
    const newPast = past.slice(0, -1);

    set({
      past: newPast,
      future: [previous, ...future],
      canUndo: newPast.length > 0,
      canRedo: true,
    });

    return previous;
  },

  redo: () => {
    const { past, future } = get();
    if (future.length === 0) return null;

    const next = future[0];
    const newFuture = future.slice(1);

    set({
      past: [...past, next],
      future: newFuture,
      canUndo: true,
      canRedo: newFuture.length > 0,
    });

    return next;
  },

  clear: () => {
    set({ past: [], future: [], canUndo: false, canRedo: false });
  },
}));

// Hook to use editor history
// eslint-disable-next-line react-refresh/only-export-components
export function useEditorHistory() {
  const { nodes, edges } = useEditorStore();
  const { pushState, undo: doUndo, redo: doRedo, canUndo, canRedo, past } = useEditorHistoryStore();

  const lastStateRef = useRef<string>('');

  useEffect(() => {
    const stateKey = JSON.stringify({ nodes, edges });
    if (stateKey !== lastStateRef.current && (nodes.length > 0 || edges.length > 0)) {
      lastStateRef.current = stateKey;
      pushState({ nodes, edges });
    }
  }, [nodes, edges, pushState]);

  const undo = useCallback(() => {
    const state = doUndo();
    if (state) {
      useEditorStore.setState({
        nodes: state.nodes as typeof nodes,
        edges: state.edges as typeof edges,
        isDirty: true,
      });
    }
  }, [doUndo]);

  const redo = useCallback(() => {
    const state = doRedo();
    if (state) {
      useEditorStore.setState({
        nodes: state.nodes as typeof nodes,
        edges: state.edges as typeof edges,
        isDirty: true,
      });
    }
  }, [doRedo]);

  return { undo, redo, canUndo, canRedo, historyLength: past.length };
}
