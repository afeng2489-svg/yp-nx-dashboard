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
    label: '智能体',
    description: '具有角色和模型的 AI 智能体',
  },
  {
    type: 'stage',
    label: '阶段',
    description: '串行或并行编排智能体',
  },
  {
    type: 'condition',
    label: '条件',
    description: '基于表达式的分支判断',
  },
  {
    type: 'loop',
    label: '循环',
    description: '重复执行工作流步骤',
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
        节点面板
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
          快速提示
        </h4>
        <ul className="text-xs text-muted-foreground space-y-1">
          <li>• 拖拽节点到画布</li>
          <li>• 通过连接点连线</li>
          <li>• 点击以选中</li>
          <li>• Backspace 键删除</li>
        </ul>
      </div>
    </div>
  );
}