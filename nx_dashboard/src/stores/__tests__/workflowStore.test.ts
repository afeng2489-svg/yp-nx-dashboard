import { describe, it, expect, beforeEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useWorkflowStore, Workflow } from '../workflowStore'

const mockFetch = vi.fn()
global.fetch = mockFetch

describe('useWorkflowStore', () => {
  const mockWorkflowSummary = {
    id: 'workflow-1',
    name: 'Test Workflow',
    version: '1.0.0',
    description: 'A test workflow',
    stage_count: 1,
    agent_count: 1,
  }

  const mockWorkflowDetail = {
    id: 'workflow-1',
    name: 'Test Workflow',
    version: '1.0.0',
    description: 'A test workflow',
    definition: {
      stages: [
        { name: 'stage1', agents: ['agent1'], parallel: false },
      ],
      agents: [
        {
          id: 'agent1',
          role: 'planner',
          model: 'claude-3-5-sonnet',
          prompt: 'You are a planner',
          depends_on: [],
        },
      ],
      triggers: [],
    },
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  }

  const mockFullWorkflow: Workflow = {
    id: 'workflow-1',
    name: 'Test Workflow',
    version: '1.0.0',
    description: 'A test workflow',
    stages: [
      { name: 'stage1', agents: ['agent1'], parallel: false },
    ],
    agents: [
      {
        id: 'agent1',
        role: 'planner',
        model: 'claude-3-5-sonnet',
        prompt: 'You are a planner',
        depends_on: [],
      },
    ],
    triggers: [],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  }

  beforeEach(() => {
    mockFetch.mockReset()
    useWorkflowStore.setState({
      workflows: [],
      currentWorkflow: null,
      loading: false,
      error: null,
    })
  })

  describe('initial state', () => {
    it('should have empty workflows array', () => {
      const { result } = renderHook(() => useWorkflowStore())
      expect(result.current.workflows).toEqual([])
    })

    it('should have null currentWorkflow', () => {
      const { result } = renderHook(() => useWorkflowStore())
      expect(result.current.currentWorkflow).toBeNull()
    })

    it('should have loading false', () => {
      const { result } = renderHook(() => useWorkflowStore())
      expect(result.current.loading).toBe(false)
    })

    it('should have null error', () => {
      const { result } = renderHook(() => useWorkflowStore())
      expect(result.current.error).toBeNull()
    })
  })

  describe('fetchWorkflows', () => {
    it('should fetch workflows successfully', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve([mockWorkflowSummary]),
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        await result.current.fetchWorkflows()
      })

      expect(result.current.workflows.length).toBe(1)
      expect(result.current.workflows[0]).toEqual({
        id: 'workflow-1',
        name: 'Test Workflow',
        version: '1.0.0',
        description: 'A test workflow',
        stages: [],
        agents: [],
        stage_count: 1,
        agent_count: 1,
        created_at: undefined,
        updated_at: undefined,
      })
      expect(result.current.loading).toBe(false)
    })

    it('should handle empty workflows', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve([]),
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        await result.current.fetchWorkflows()
      })

      expect(result.current.workflows).toEqual([])
      expect(result.current.loading).toBe(false)
    })

    it('should handle fetch error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        await result.current.fetchWorkflows()
      })

      expect(result.current.workflows).toEqual([])
      expect(result.current.loading).toBe(false)
      expect(result.current.error).not.toBeNull()
    })
  })

  describe('getWorkflow', () => {
    it('should get a workflow by id', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: () => Promise.resolve(mockWorkflowDetail),
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        const workflow = await result.current.getWorkflow('workflow-1')
        expect(workflow).toEqual(mockFullWorkflow)
      })
    })

    it('should return null for 404', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        const workflow = await result.current.getWorkflow('non-existent')
        expect(workflow).toBeNull()
      })
    })
  })

  describe('createWorkflow', () => {
    it('should create a new workflow', async () => {
      const newWorkflow = { ...mockFullWorkflow, id: 'workflow-2', name: 'New Workflow' }
      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: () => Promise.resolve(newWorkflow),
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        const workflow = await result.current.createWorkflow({
          name: 'New Workflow',
          version: '1.0.0',
          stages: [],
          agents: [],
        })
        expect(workflow).toEqual(newWorkflow)
      })

      expect(result.current.workflows).toContainEqual(newWorkflow)
    })

    it('should handle create error', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        try {
          await result.current.createWorkflow({
            name: 'Bad Workflow',
            version: '1.0.0',
            stages: [],
            agents: [],
          })
        } catch {
          // Expected to throw
        }
      })

      expect(result.current.error).not.toBeNull()
    })
  })

  describe('updateWorkflow', () => {
    it('should update a workflow', async () => {
      useWorkflowStore.setState({ workflows: [mockFullWorkflow] })

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        await result.current.updateWorkflow('workflow-1', {
          name: 'Updated Workflow',
        })
      })

      const updated = result.current.workflows.find(w => w.id === 'workflow-1')
      expect(updated?.name).toBe('Updated Workflow')
    })
  })

  describe('deleteWorkflow', () => {
    it('should delete a workflow', async () => {
      useWorkflowStore.setState({ workflows: [mockFullWorkflow] })

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 204,
      })

      const { result } = renderHook(() => useWorkflowStore())

      await act(async () => {
        await result.current.deleteWorkflow('workflow-1')
      })

      expect(result.current.workflows).not.toContainEqual(
        expect.objectContaining({ id: 'workflow-1' })
      )
    })
  })

  describe('setCurrentWorkflow', () => {
    it('should set current workflow', () => {
      const { result } = renderHook(() => useWorkflowStore())

      act(() => {
        result.current.setCurrentWorkflow(mockFullWorkflow)
      })

      expect(result.current.currentWorkflow).toEqual(mockFullWorkflow)
    })

    it('should clear current workflow with null', () => {
      useWorkflowStore.setState({ currentWorkflow: mockFullWorkflow })

      const { result } = renderHook(() => useWorkflowStore())

      act(() => {
        result.current.setCurrentWorkflow(null)
      })

      expect(result.current.currentWorkflow).toBeNull()
    })
  })

  describe('clearError', () => {
    it('should clear error', () => {
      useWorkflowStore.setState({ error: 'Some error' })

      const { result } = renderHook(() => useWorkflowStore())

      act(() => {
        result.current.clearError()
      })

      expect(result.current.error).toBeNull()
    })
  })
})
