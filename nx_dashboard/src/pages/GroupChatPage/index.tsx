import { Plus, MessageSquare, Loader2, AlertCircle, CheckCircle, X } from 'lucide-react';
import { ConfirmModal } from '@/lib/ConfirmModal';
import { useGroupChatPage } from './hooks/useGroupChatPage';
import { CreateSessionModal } from './CreateSessionModal';
import { StartDiscussionModal } from './StartDiscussionModal';
import { ConclusionModal } from './ConclusionModal';
import { ChatMessageList } from './ChatMessageList';
import { ChatInput } from './ChatInput';
import { TeamEvolutionSection } from './TeamEvolutionSection';
import { SessionsList } from './SessionsList';
import { SessionHeader } from './SessionHeader';
import { ParallelRoundProgress } from './ParallelRoundProgress';

export function GroupChatPage() {
  const page = useGroupChatPage();

  if (page.loading && page.sessions.length === 0) {
    return (
      <div className="page-container">
        <div className="flex items-center justify-center h-64">
          <Loader2 className="w-8 h-8 animate-spin text-primary" />
        </div>
      </div>
    );
  }

  return (
    <div className="page-container">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <MessageSquare className="w-6 h-6" />
          <h1 className="text-2xl font-bold">团队群组讨论</h1>
        </div>
        <button
          onClick={() => page.setShowCreateModal(true)}
          className="btn btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          新建讨论
        </button>
      </div>

      {/* Error display */}
      {page.error && (
        <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-4 mb-4 flex items-center gap-3">
          <AlertCircle className="w-5 h-5 text-destructive" />
          <span className="flex-1">{page.error}</span>
          <button onClick={page.clearError} className="p-1 hover:bg-destructive/20 rounded">
            <X className="w-4 h-4" />
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

        <div className="col-span-8 space-y-4">
          {page.selectedSessionId && page.currentSession ? (
            <>
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
              />

              <ParallelRoundProgress bots={page.parallelRound.bots} />

              {page.currentWorkspace?.id && (
                <TeamEvolutionSection projectId={page.currentWorkspace.id} />
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
              />

              {page.currentSession.status === 'active' && (
                <ChatInput onSend={page.handleSendMessage} disabled={false} skills={page.skills} />
              )}

              {page.currentSession.conclusion && (
                <div className="bg-card rounded-lg border p-4">
                  <h3 className="font-semibold flex items-center gap-2 mb-3">
                    <CheckCircle className="w-4 h-4 text-green-500" />
                    讨论结论
                  </h3>
                  <p className="text-sm whitespace-pre-wrap">
                    {page.currentSession.conclusion.content}
                  </p>
                  <div className="mt-3 pt-3 border-t flex items-center gap-4 text-xs text-muted-foreground">
                    <span>
                      共识度: {(page.currentSession.conclusion.consensus_level * 100).toFixed(0)}%
                    </span>
                    <span>同意人数: {page.currentSession.conclusion.agreed_by.length}</span>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="bg-card rounded-lg border flex items-center justify-center h-[400px]">
              <div className="text-center text-muted-foreground">
                <MessageSquare className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>选择一个讨论会话查看详情</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Modals */}
      <CreateSessionModal
        isOpen={page.showCreateModal}
        onClose={() => page.setShowCreateModal(false)}
        teams={page.teams}
        createForm={page.createForm}
        onFormChange={page.setCreateForm}
        onSubmit={page.handleCreateSession}
      />

      {page.currentSession && (
        <StartDiscussionModal
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
    </div>
  );
}
