import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TeamSessionsPage } from '../TeamSessionsPage';

// ── Mocks ──────────────────────────────────────────────────────────

// Mock Tauri API event listener
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

// Mock Tauri API core invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));

// Mock API_BASE_URL
vi.mock('@/api/constants', () => ({
  API_BASE_URL: '',
}));

// ── Fixtures ───────────────────────────────────────────────────────

const mockSessions = [
  {
    execution_id: '550e8400-e29b-41d4-a716-446655440000',
    task: '实现用户登录功能',
    status: 'completed',
    agent_count: 4,
    duration_ms: 45000,
    created_at: '2026-05-14T10:30:00',
  },
  {
    execution_id: '550e8400-e29b-41d4-a716-446655440001',
    task: '重构 auth 模块',
    status: 'failed',
    agent_count: 2,
    duration_ms: 12000,
    created_at: '2026-05-14T09:15:00',
  },
  {
    execution_id: '550e8400-e29b-41d4-a716-446655440002',
    task: '添加数据库迁移脚本',
    status: 'completed',
    agent_count: 3,
    duration_ms: 30000,
    created_at: '2026-05-13T18:00:00',
  },
];

const mockDetail = {
  execution_id: '550e8400-e29b-41d4-a716-446655440000',
  task: '实现用户登录功能',
  agent_results: [
    {
      agent_id: 'a1',
      role: 'architect',
      agent_name: '架构师',
      text: '# 架构设计\n模块划分：...',
      duration_ms: 15000,
      attempts: 1,
    },
    {
      agent_id: 'a2',
      role: 'developer',
      agent_name: '开发者',
      text: '```rust\nfn login() {}\n```',
      duration_ms: 20000,
      attempts: 2,
    },
  ],
  total_duration_ms: 45000,
};

// ── Helpers ────────────────────────────────────────────────────────

function createMockFetch(ok: boolean, data: unknown) {
  return vi.fn().mockResolvedValue({
    ok,
    json: () => Promise.resolve(data),
  });
}

function renderPage() {
  return render(<TeamSessionsPage />);
}

// ── Tests ──────────────────────────────────────────────────────────

describe('TeamSessionsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn();
  });

  describe('loading state', () => {
    it('shows spinner while loading', () => {
      // Never resolve fetch so loading persists
      globalThis.fetch = vi.fn(() => new Promise(() => {}));

      renderPage();
      const spinner = document.querySelector('.animate-spin');
      expect(spinner).toBeTruthy();
    });
  });

  describe('empty state', () => {
    beforeEach(() => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: [] });
    });

    it('shows empty message when no sessions', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('暂无团队会话记录')).toBeTruthy();
      });
    });

    it('shows new session button in empty state', async () => {
      renderPage();

      await waitFor(() => {
        const buttons = screen.getAllByText('新建会话');
        expect(buttons.length).toBeGreaterThanOrEqual(1);
      });
    });
  });

  describe('session list', () => {
    beforeEach(() => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
    });

    it('renders session cards', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
        expect(screen.getByText('重构 auth 模块')).toBeTruthy();
        expect(screen.getByText('添加数据库迁移脚本')).toBeTruthy();
      });
    });

    it('shows agent count for each session', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText(/4 个智能体/)).toBeTruthy();
        expect(screen.getByText(/2 个智能体/)).toBeTruthy();
        expect(screen.getByText(/3 个智能体/)).toBeTruthy();
      });
    });

    it('shows status icons', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getAllByText('已完成').length).toBe(2);
        expect(screen.getByText('失败')).toBeTruthy();
      });
    });

    it('shows total session count in header', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText(/共 3 个会话/)).toBeTruthy();
      });
    });
  });

  describe('detail modal', () => {
    beforeEach(() => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
    });

    it('opens detail modal on session click', async () => {
      renderPage();

      // Wait for sessions to load
      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
      });

      // Set up detail mock BEFORE clicking
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockDetail });

      // Click the session
      await userEvent.click(screen.getByText('实现用户登录功能'));

      await waitFor(() => {
        expect(screen.getByText('会话详情')).toBeTruthy();
      });
    });

    it('shows agent results in detail', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
      });

      // Set up detail mock BEFORE clicking
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockDetail });

      // Click to open detail
      await userEvent.click(screen.getByText('实现用户登录功能'));

      await waitFor(() => {
        expect(screen.getByText('智能体执行结果')).toBeTruthy();
      });
    });
  });

  describe('create session modal', () => {
    beforeEach(() => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
    });

    it('opens create modal when clicking new session button', async () => {
      renderPage();

      await waitFor(() => {
        const buttons = screen.getAllByText('新建会话');
        expect(buttons.length).toBeGreaterThanOrEqual(1);
      });

      await userEvent.click(screen.getAllByText('新建会话')[0]);

      await waitFor(() => {
        expect(screen.getByText('新建团队会话')).toBeTruthy();
      });
    });

    it('shows task description textarea', async () => {
      renderPage();

      await waitFor(() => {
        const buttons = screen.getAllByText('新建会话');
        userEvent.click(buttons[0]);
      });

      await waitFor(() => {
        const textarea = screen.getByPlaceholderText(/例如：/);
        expect(textarea).toBeTruthy();
      });
    });

    it('disables submit button when task is empty', async () => {
      renderPage();

      await waitFor(() => {
        const buttons = screen.getAllByText('新建会话');
        userEvent.click(buttons[0]);
      });

      await waitFor(() => {
        const submitBtn = screen.getByText('启动会话');
        expect(submitBtn.closest('button')).toBeDisabled();
      });
    });
  });

  describe('delete confirmation', () => {
    beforeEach(() => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
    });

    it('shows delete confirmation on delete button click', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
      });

      // Find and click delete button (trash icon buttons have title "删除会话")
      const deleteButtons = screen.getAllByTitle('删除会话');
      await userEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(screen.getByText('确认删除')).toBeTruthy();
        expect(screen.getByText(/永久删除/)).toBeTruthy();
      });
    });

    it('closes delete modal on cancel', async () => {
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
      });

      // Open delete dialog
      const deleteButtons = screen.getAllByTitle('删除会话');
      await userEvent.click(deleteButtons[0]);

      await waitFor(() => {
        expect(screen.getByText('确认删除')).toBeTruthy();
      });

      // Click cancel
      await userEvent.click(screen.getByText('取消'));

      await waitFor(() => {
        expect(screen.queryByText('确认删除')).toBeNull();
      });
    });
  });

  describe('error handling', () => {
    it('handles fetch failure gracefully', async () => {
      globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

      renderPage();

      // Should not crash, should show loading then empty state or error
      await waitFor(() => {
        // Component catches the error and sets loading to false
        const spinner = document.querySelector('.animate-spin');
        expect(spinner).toBeFalsy();
      });
    });

    it('handles non-ok response from list endpoint', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        json: () => Promise.resolve({ ok: false, error: 'Internal error' }),
      });

      renderPage();

      // Should not crash
      await waitFor(() => {
        const spinner = document.querySelector('.animate-spin');
        expect(spinner).toBeFalsy();
      });
    });

    it('handles API response with ok:false', async () => {
      globalThis.fetch = createMockFetch(true, {
        ok: false,
        error: 'Server error',
      });

      renderPage();

      await waitFor(() => {
        const spinner = document.querySelector('.animate-spin');
        expect(spinner).toBeFalsy();
      });
    });
  });

  describe('pagination', () => {
    function generateSessions(count: number) {
      return Array.from({ length: count }, (_, i) => ({
        execution_id: `id-${i}`,
        task: `Session ${i + 1}`,
        status: 'completed',
        agent_count: 1,
        duration_ms: 1000,
        created_at: '2026-05-14T10:00:00',
      }));
    }

    it('paginates when more than PAGE_SIZE sessions', async () => {
      const manySessions = generateSessions(20);
      globalThis.fetch = createMockFetch(true, { ok: true, data: manySessions });

      renderPage();

      await waitFor(() => {
        // 20 sessions with PAGE_SIZE=8 = 3 pages
        expect(screen.getByText('Session 1')).toBeTruthy();
        // Should show pagination controls
        expect(screen.getByText('3')).toBeTruthy(); // total 3 pages
      });
    });
  });

  describe('edge cases', () => {
    it('handles empty agent_results in detail modal', async () => {
      const emptyDetail = {
        execution_id: 'empty-id',
        task: 'task with no results',
        agent_results: [],
        total_duration_ms: 0,
      };

      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
      renderPage();

      await waitFor(() => {
        expect(screen.getByText('实现用户登录功能')).toBeTruthy();
      });

      globalThis.fetch = createMockFetch(true, { ok: true, data: emptyDetail });
      await userEvent.click(screen.getByText('实现用户登录功能'));

      await waitFor(() => {
        expect(screen.getByText('任务')).toBeTruthy();
        expect(screen.getByText('智能体数')).toBeTruthy();
      });
    });

    it('handles empty task string in session list', async () => {
      const sessionsWithEmptyTask = [{ ...mockSessions[0], task: '' }];
      globalThis.fetch = createMockFetch(true, { ok: true, data: sessionsWithEmptyTask });
      renderPage();

      await waitFor(() => {
        expect(screen.getByText(/共 1 个会话/)).toBeTruthy();
      });
    });

    it('handles very long task text without overflow', async () => {
      const longTask = 'A'.repeat(1000);
      const sessionsWithLongTask = [{ ...mockSessions[0], task: longTask }];
      globalThis.fetch = createMockFetch(true, { ok: true, data: sessionsWithLongTask });
      renderPage();

      await waitFor(() => {
        const taskElement = screen.getByText(longTask);
        expect(taskElement).toBeTruthy();
        expect(taskElement.classList.contains('truncate')).toBeTruthy();
      });
    });

    it('handles special characters in task text', async () => {
      const specialTask = '<script>alert("xss")</script> & "quotes" \'单引号\'';
      const sessionsWithSpecial = [{ ...mockSessions[0], task: specialTask }];
      globalThis.fetch = createMockFetch(true, { ok: true, data: sessionsWithSpecial });
      renderPage();

      await waitFor(() => {
        expect(screen.getByText(specialTask)).toBeTruthy();
      });
    });

    it('disables submit when task has only whitespace', async () => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
      renderPage();

      await waitFor(() => {
        userEvent.click(screen.getAllByText('新建会话')[0]);
      });

      await waitFor(() => {
        const textarea = screen.getByPlaceholderText(/例如：/);
        userEvent.type(textarea, '   ');
      });

      await waitFor(() => {
        const submitBtn = screen.getByText('启动会话');
        expect(submitBtn.closest('button')).toBeDisabled();
      });
    });

    it('shows error message in create modal on invoke failure', async () => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: mockSessions });
      renderPage();

      await waitFor(() => {
        userEvent.click(screen.getAllByText('新建会话')[0]);
      });

      // Wait for modal then type a task
      await waitFor(() => {
        expect(screen.getByPlaceholderText(/例如：/)).toBeTruthy();
      });
      const textarea = screen.getByPlaceholderText(/例如：/);
      await userEvent.type(textarea, 'test task');

      // Mock Tauri invoke to fail
      const invokeMock = vi.fn().mockRejectedValue(new Error('Tauri invoke failed'));
      const coreModule = await import('@tauri-apps/api/core');
      (coreModule.invoke as ReturnType<typeof vi.fn>).mockImplementation(invokeMock);

      // Click submit
      const submitBtn = screen.getByText('启动会话');
      await userEvent.click(submitBtn);

      await waitFor(() => {
        expect(screen.getByText(/Tauri invoke failed/)).toBeTruthy();
      });
    });

    it('handles null/undefined from API gracefully', async () => {
      globalThis.fetch = createMockFetch(true, { ok: true, data: null });
      renderPage();

      await waitFor(() => {
        const spinner = document.querySelector('.animate-spin');
        expect(spinner).toBeFalsy();
      });
    });

    it('handles non-JSON API response gracefully', async () => {
      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.reject(new Error('Invalid JSON')),
      });
      renderPage();

      await waitFor(() => {
        const spinner = document.querySelector('.animate-spin');
        expect(spinner).toBeFalsy();
      });
    });
  });
});
