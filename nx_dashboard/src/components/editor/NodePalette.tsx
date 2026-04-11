import { NodeType } from '@/stores/editorStore';
import { NODE_COLORS, NODE_ICONS } from './types';

interface PaletteItem {
  type: NodeType;
  label: string;
  description: string;
}

const paletteItems: PaletteItem[] = [
  {
    type: 'agent',
    label: 'Agent',
    description: 'AI agent with role and model',
  },
  {
    type: 'stage',
    label: 'Stage',
    description: 'Group agents in parallel/sequential',
  },
  {
    type: 'condition',
    label: 'Condition',
    description: 'Branch based on expression',
  },
  {
    type: 'loop',
    label: 'Loop',
    description: 'Repeat workflow steps',
  },
];

export function NodePalette() {
  const handleDragStart = (
    event: React.DragEvent,
    nodeType: NodeType
  ) => {
    event.dataTransfer.setData('application/reactflow', nodeType);
    event.dataTransfer.effectAllowed = 'move';
  };

  return (
    <div className="w-64 bg-card border border-border rounded-lg shadow-md p-4">
      <h3 className="font-semibold text-sm mb-3 text-foreground">
        Node Palette
      </h3>
      <div className="space-y-2">
        {paletteItems.map((item) => {
          const color = NODE_COLORS[item.type];
          const icon = NODE_ICONS[item.type];
          return (
            <div
              key={item.type}
              draggable
              onDragStart={(e) => handleDragStart(e, item.type)}
              className="
                p-3 rounded-lg border-2 cursor-grab active:cursor-grabbing
                transition-all hover:shadow-md
                bg-white hover:bg-gray-50
                dark:bg-card dark:hover:bg-accent
              "
              style={{
                borderColor: color,
              }}
            >
              <div className="flex items-center gap-2">
                <span className="text-lg">{icon}</span>
                <div className="flex flex-col">
                  <span className="font-medium text-sm">{item.label}</span>
                  <span className="text-xs text-muted-foreground">
                    {item.description}
                  </span>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <div className="mt-6 pt-4 border-t border-border">
        <h4 className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wide">
          Quick Tips
        </h4>
        <ul className="text-xs text-muted-foreground space-y-1">
          <li>• Drag nodes to canvas</li>
          <li>• Connect via handles</li>
          <li>• Click to select</li>
          <li>• Delete with Backspace</li>
        </ul>
      </div>
    </div>
  );
}