import { Plus, MessagesSquare, Loader2, AlertCircle, CheckCircle, X } from 'lucide-react';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { useGroupChatPage } from './hooks/useGroupChatPage';
import { CreateSessionModal } from './CreateSessionModal';
import { DiscussionSetupSheet } from './DiscussionSetupSheet';
import { ConclusionModal } from './ConclusionModal';
import { ChatMessageList } from './ChatMessageList';
import { ChatInput } from './ChatInput';
import { TeamEvolutionSection } from './TeamEvolutionSection';
import { SessionsList } from './SessionsList';
import { SessionHeader } from './SessionHeader';
import { ParallelRoundProgress } from './ParallelRoundProgress';
import { DiscussionEmptyState } from './DiscussionEmptyState';
import { cn } from '@/lib/utils';

export function GroupChatPage({
  embedded = false,
  teamId,
}: {
  embedded?: boolean;
  teamId?: string;
} = {}) {
  const page = useGroupChatPage(teamId);
  const openCreate = () => page.setShowCreateModal(true);

  if (page.loading && page.sessions.length === 0) {
    return (
      <div
        className={cn(
          'flex items-center justify-center',
          embedded ? 'h-48' : 'page-container min-h-[320px]',
        )}
      >
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  const mainPanel =
    page.selectedSessionId && page.currentSession ? (
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
        <SessionHeader
          currentSession={page.currentSession}
          nextSpeaker={page.nextSpeaker}
          autoMode={page.autoMode}
          isRoundRunning={page.isRoundRunning}
          isAgentActive={page.isAgentActive}
          executingRole={page.executingRole}
          onStartDiscussion={() => page.setShowStartModal(true)}
          onToggleAutoMode={() => page.setAutoMode(!page.autoMode)}
          onExecuteRound={page.handleExecuteRound}
          onConcludeDiscussion={() => page.setShowConclusionModal(true)}
          onExecuteRoleTurn={page.handleExecuteRoleTurn}
          compact={embedded}
        />

        <ParallelRoundProgress bots={page.parallelRound.bots} />

        {page.currentWorkspace?.id && (
          <TeamEvolutionSection workspaceId={page.currentWorkspace.id} />
        )}

        <ChatMessageList
          messages={page.messages}
          selectedSessionId={page.selectedSessionId}
          currentSession={page.currentSession}
          executingRole={page.executingRole}
          isAgentActive={page.isAgentActive}
          agentExec={page.agentExec}
          onRefresh={() => page.fetchMessages(page.selectedSessionId!)}
          onCancelExecution={page.handleCancelExecution}
          embedded={embedded}
        />

        {page.currentSession.status === 'active' && (
          <ChatInput
            onSend={page.handleSendMessage}
            disabled={false}
            skills={page.skills}
            roles={page.roles[page.currentSession.team_id] ?? []}
          />
        )}

        {page.currentSession.conclusion && (
          <div
            className="rounded-xl border border-green-500/20 bg-green-500/5 p-5 shadow-sm"
            data-testid="discussion-conclusion"
          >
            <h3 className="mb-3 flex items-center gap-2 font-semibold">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-green-500/15">
                <CheckCircle className="h-4 w-4 text-green-500" />
              </span>
              讨论结论
            </h3>
            <p className="whitespace-pre-wrap text-sm leading-relaxed">
              {page.currentSession.conclusion.content}
            </p>
            <div className="mt-4 flex flex-wrap items-center gap-3 border-t border-green-500/15 pt-3 text-xs text-muted-foreground">
              <span className="flex items-center gap-1.5">
                共识度
                <span className="font-semibold tabular-nums text-foreground">
                  {(page.currentSession.conclusion.consensus_level * 100).toFixed(0)}%
                </span>
              </span>
              <span className="flex items-center gap-1.5">
                同意人数
                <span className="font-semibold tabular-nums text-foreground">
                  {page.currentSession.conclusion.agreed_by.length}
                </span>
              </span>
              <a
                href={`/factory?tab=console&intent=${encodeURIComponent(page.currentSession.conclusion.content.slice(0, 500))}`}
                className="btn btn-primary ml-auto px-3 py-1 text-xs"
                data-testid="conclusion-launch-run"
              >
                按此启动 Run
              </a>
            </div>
          </div>
        )}
      </div>
    ) : (
      <div className="flex min-h-0 flex-1 items-center justify-center bg-background/50">
        <DiscussionEmptyState
          icon={<MessagesSquare className="h-7 w-7" />}
          title={page.sessions.length === 0 ? '还没有讨论会话' : '选择一个讨论会话'}
          description={
            page.sessions.length === 0
              ? '点击上方「新建讨论」，开启第一个多 Agent 协作会话'
              : '从左侧列表选择已有讨论，查看对话与结论'
          }
        />
      </div>
    );

  const modals = (
    <>
      <CreateSessionModal
        isOpen={page.showCreateModal}
        onClose={() => page.setShowCreateModal(false)}
        teams={page.teams}
        createForm={page.createForm}
        onFormChange={page.setCreateForm}
        onSubmit={page.handleCreateSession}
      />

      {page.currentSession && (
        <DiscussionSetupSheet
          isOpen={page.showStartModal}
          onClose={() => page.setShowStartModal(false)}
          currentSession={page.currentSession}
          roles={page.roles}
          startForm={page.startForm}
          onFormChange={page.setStartForm}
          onSubmit={page.handleStartDiscussion}
        />
      )}

      <ConclusionModal
        isOpen={page.showConclusionModal}
        onClose={() => page.setShowConclusionModal(false)}
        onConclude={page.handleConcludeDiscussion}
      />

      {page.confirmState.isOpen && (
        <ConfirmModal
          isOpen={page.confirmState.isOpen}
          title={page.confirmState.title}
          message={page.confirmState.message}
          onConfirm={() => {
            page.confirmState.onConfirm?.();
            page.hideConfirm();
          }}
          onCancel={page.hideConfirm}
          variant={page.confirmState.variant || 'danger'}
        />
      )}
    </>
  );

  if (embedded) {
    return (
      <div className="flex h-full min-h-[480px] flex-col" data-testid="group-chat-embedded">
        <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border/60 bg-muted/20 px-4 py-3">
          <div className="flex min-w-0 items-center gap-3">
            <div className="relative shrink-0 text-muted-foreground">
              <MessagesSquare className="h-5 w-5" />
              <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full bg-indigo-500 ring-2 ring-background" />
            </div>
            <div className="min-w-0">
              <h2 className="text-sm font-semibold tracking-tight">多 Agent 讨论</h2>
              <p className="truncate text-xs text-muted-foreground">
                多角色协作讨论，产出共识结论
              </p>
            </div>
          </div>
          <button type="button" onClick={openCreate} className="btn btn-primary h-9 shrink-0 px-3 text-sm">
            <Plus className="h-4 w-4" />
            新建讨论
          </button>
        </div>

        {page.error && (
          <div className="mx-4 mt-3 flex shrink-0 items-center gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-3">
            <AlertCircle className="h-4 w-4 shrink-0 text-destructive" />
            <span className="flex-1 text-sm">{page.error}</span>
            <button type="button" onClick={page.clearError} className="btn-icon h-8 w-8">
              <X className="h-4 w-4" />
            </button>
          </div>
        )}

        <div className="flex min-h-0 flex-1">
          <SessionsList
            embedded
            sessions={page.sessions}
            selectedSessionId={page.selectedSessionId}
            onSelectSession={page.setSelectedSessionId}
            onDeleteSession={page.handleDeleteSession}
          />
          <main className="flex min-w-0 flex-1 flex-col">{mainPanel}</main>
        </div>

        {modals}
      </div>
    );
  }

  return (
    <div className="page-container">
      <div className="mb-6 flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="relative text-muted-foreground">
            <MessagesSquare className="h-6 w-6" />
            <span className="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full bg-indigo-500 ring-2 ring-background" />
          </div>
          <div>
            <h1 className="text-2xl font-bold tracking-tight">团队群组讨论</h1>
            <p className="text-sm text-muted-foreground">多 Agent 协作讨论，产出可执行的共识结论</p>
          </div>
        </div>
        <button type="button" onClick={openCreate} className="btn btn-primary h-11 px-5">
          <Plus className="h-4 w-4" />
          新建讨论
        </button>
      </div>

      {page.error && (
        <div className="mb-4 flex items-center gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-4">
          <AlertCircle className="h-5 w-5 text-destructive" />
          <span className="flex-1">{page.error}</span>
          <button type="button" onClick={page.clearError} className="btn-icon">
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      <div className="grid grid-cols-12 gap-6">
        <SessionsList
          sessions={page.sessions}
          selectedSessionId={page.selectedSessionId}
          onSelectSession={page.setSelectedSessionId}
          onDeleteSession={page.handleDeleteSession}
        />
        <div className="col-span-8 flex min-h-[440px] flex-col rounded-xl border border-border/60 bg-card shadow-sm">
          {mainPanel}
        </div>
      </div>

      {modals}
    </div>
  );
}
