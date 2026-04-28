import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeHighlight from 'rehype-highlight';
import 'highlight.js/styles/github-dark.css';
import { memo } from 'react';

interface MarkdownMessageProps {
  content: string;
  /** 'user' 用主色调（白字深底），'assistant' 用前景色 */
  variant?: 'user' | 'assistant';
}

/**
 * AI 消息 Markdown 渲染。
 * - GFM 表格 / 任务列表 / 删除线
 * - 代码块语法高亮（github-dark 主题）
 * - 链接打开在新标签
 */
export const MarkdownMessage = memo(function MarkdownMessage({
  content,
  variant = 'assistant',
}: MarkdownMessageProps) {
  const proseColor =
    variant === 'user'
      ? 'prose-invert prose-headings:text-white prose-strong:text-white prose-code:text-white'
      : '';

  return (
    <div className={`prose prose-sm dark:prose-invert max-w-none break-words ${proseColor}`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
          // 链接默认在新标签打开
          a: ({ node: _node, ...props }) => (
            <a {...props} target="_blank" rel="noopener noreferrer" />
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});
