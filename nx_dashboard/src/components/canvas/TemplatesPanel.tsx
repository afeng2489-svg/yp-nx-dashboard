import { useCanvasStore } from '@/stores/canvasStore';
import { toast } from 'sonner';

const TEMPLATES = [
  {
    name: '全栈功能开发',
    desc: '需求分析 → 后端API → 前端页面 → 测试 → 代码审查',
    yaml: `name: 全栈功能开发
version: "1.0"
stages:
  - name: 需求分析
    type: agent
    model: claude-opus-4-7
    system_prompt: 分析需求，输出技术方案
  - name: 后端API
    type: agent
    model: claude-opus-4-7
    system_prompt: 实现后端API
  - name: 前端页面
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 实现前端页面
  - name: 测试
    type: quality_gate
    checks: [cargo test, npm test]
    on_fail: retry
    max_retries: 2
  - name: 代码审查
    type: agent
    model: claude-opus-4-7
    system_prompt: 审查代码质量
`,
  },
  {
    name: 'Bug 修复流水线',
    desc: '问题分析 → 定位代码 → 修复 → 测试验证 → 生成报告',
    yaml: `name: Bug修复流水线
version: "1.0"
stages:
  - name: 问题分析
    type: agent
    model: claude-opus-4-7
    system_prompt: 分析bug原因
  - name: 定位代码
    type: shell
    command: grep -rn "{{keyword}}" src/
  - name: 修复
    type: agent
    model: claude-opus-4-7
    system_prompt: 修复bug
  - name: 测试验证
    type: quality_gate
    checks: [cargo test]
    on_fail: fail
  - name: 生成报告
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 生成修复报告
`,
  },
  {
    name: '代码审查流水线',
    desc: '读取PR diff → 安全检查 → 性能检查 → 代码质量 → 审查报告',
    yaml: `name: 代码审查流水线
version: "1.0"
stages:
  - name: 读取PR
    type: shell
    command: git diff HEAD~1
  - name: 安全检查
    type: agent
    model: claude-opus-4-7
    system_prompt: 检查安全漏洞
  - name: 性能检查
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 检查性能问题
  - name: 代码质量
    type: quality_gate
    checks: [cargo clippy -- -D warnings]
    on_fail: continue
  - name: 审查报告
    type: agent
    model: claude-opus-4-7
    system_prompt: 生成审查报告
`,
  },
  {
    name: '文档生成流水线',
    desc: '读取代码 → 生成API文档 → 使用示例 → README',
    yaml: `name: 文档生成流水线
version: "1.0"
stages:
  - name: 读取代码
    type: shell
    command: find src -name "*.rs" | head -20
  - name: 生成API文档
    type: agent
    model: claude-opus-4-7
    system_prompt: 生成API文档
  - name: 使用示例
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 生成使用示例
  - name: 生成README
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 生成README
`,
  },
  {
    name: '数据处理流水线',
    desc: '读取数据 → 清洗 → 分析 → 生成报告 → 发送通知',
    yaml: `name: 数据处理流水线
version: "1.0"
stages:
  - name: 读取数据
    type: http
    method: GET
    url: https://api.example.com/data
  - name: 数据清洗
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 清洗数据
  - name: 数据分析
    type: agent
    model: claude-opus-4-7
    system_prompt: 分析数据
  - name: 生成报告
    type: agent
    model: claude-sonnet-4-6
    system_prompt: 生成报告
  - name: 发送通知
    type: http
    method: POST
    url: https://api.example.com/notify
`,
  },
];

export function TemplatesPanel({ onClose }: { onClose: () => void }) {
  const loadFromYaml = useCanvasStore((s) => s.loadFromYaml);

  const use = (yaml: string) => {
    loadFromYaml(yaml);
    toast.success('模板已加载到画布');
    onClose();
  };

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-[640px] max-h-[80vh] overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-900 p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold text-white">模板库</h2>
          <button onClick={onClose} className="text-zinc-500 hover:text-white text-lg">✕</button>
        </div>
        <div className="grid grid-cols-1 gap-3">
          {TEMPLATES.map((t) => (
            <div key={t.name} className="flex items-center justify-between rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-3">
              <div>
                <p className="text-sm font-medium text-white">{t.name}</p>
                <p className="text-xs text-zinc-400 mt-0.5">{t.desc}</p>
              </div>
              <button
                onClick={() => use(t.yaml)}
                className="ml-4 shrink-0 rounded bg-blue-600 px-3 py-1 text-xs text-white hover:bg-blue-500"
              >
                使用
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
