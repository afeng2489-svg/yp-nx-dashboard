import { describe, it, expect, beforeEach, vi } from 'vitest'
import { act } from '@testing-library/react'
import { useSkillStore } from '../skillStore'

const mockFetch = vi.fn()
global.fetch = mockFetch

const mockSkillSummary = {
  id: 'skill-1',
  name: 'Test Skill',
  description: 'A test skill',
  category: 'testing',
  version: '1.0.0',
  tags: ['test'],
  parameter_count: 0,
  is_preset: false,
}

const mockSkillDetail = {
  ...mockSkillSummary,
  author: null,
  parameters: [],
  code: 'Do the thing.',
  enabled: true,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

function mockOkResponse(body: unknown) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
  } as Response)
}

function mockErrorResponse(status: number) {
  return Promise.resolve({
    ok: false,
    status,
    statusText: 'Error',
    json: () => Promise.resolve({}),
  } as Response)
}

describe('useSkillStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useSkillStore.setState({
      skills: [],
      currentSkill: null,
      stats: null,
      categories: [],
      tags: [],
      searchResults: [],
      loading: false,
      saving: false,
      executing: false,
      error: null,
    })
  })

  describe('initial state', () => {
    it('has empty skills list', () => {
      expect(useSkillStore.getState().skills).toEqual([])
    })

    it('has no error', () => {
      expect(useSkillStore.getState().error).toBeNull()
    })

    it('is not loading', () => {
      expect(useSkillStore.getState().loading).toBe(false)
    })
  })

  describe('fetchSkills', () => {
    it('loads skills and clears loading on success', async () => {
      mockFetch.mockReturnValue(mockOkResponse([mockSkillSummary]))

      await act(async () => {
        await useSkillStore.getState().fetchSkills()
      })

      const state = useSkillStore.getState()
      expect(state.skills).toHaveLength(1)
      expect(state.skills[0].id).toBe('skill-1')
      expect(state.loading).toBe(false)
      expect(state.error).toBeNull()
    })

    it('sets error on network failure', async () => {
      mockFetch.mockRejectedValue(new Error('Network error'))

      await act(async () => {
        await useSkillStore.getState().fetchSkills()
      })

      const state = useSkillStore.getState()
      expect(state.loading).toBe(false)
      expect(state.error).toContain('Network error')
    })

    it('sets error on non-ok response', async () => {
      mockFetch.mockReturnValue(mockErrorResponse(500))

      await act(async () => {
        await useSkillStore.getState().fetchSkills()
      })

      expect(useSkillStore.getState().error).not.toBeNull()
    })
  })

  describe('fetchSkill', () => {
    it('returns skill detail and sets currentSkill', async () => {
      mockFetch.mockReturnValue(mockOkResponse(mockSkillDetail))

      let result: unknown
      await act(async () => {
        result = await useSkillStore.getState().fetchSkill('skill-1')
      })

      expect(result).toMatchObject({ id: 'skill-1' })
      expect(useSkillStore.getState().currentSkill?.id).toBe('skill-1')
    })

    it('returns null on 404', async () => {
      mockFetch.mockReturnValue(mockErrorResponse(404))

      let result: unknown
      await act(async () => {
        result = await useSkillStore.getState().fetchSkill('nonexistent')
      })

      expect(result).toBeNull()
      expect(useSkillStore.getState().currentSkill).toBeNull()
    })
  })

  describe('clearError', () => {
    it('clears the error state', () => {
      useSkillStore.setState({ error: 'Some error' })
      useSkillStore.getState().clearError()
      expect(useSkillStore.getState().error).toBeNull()
    })
  })

  describe('clearCurrentSkill', () => {
    it('clears currentSkill', () => {
      useSkillStore.setState({ currentSkill: mockSkillDetail })
      useSkillStore.getState().clearCurrentSkill()
      expect(useSkillStore.getState().currentSkill).toBeNull()
    })
  })

  describe('clearSearch', () => {
    it('empties searchResults', () => {
      useSkillStore.setState({ searchResults: [mockSkillSummary] })
      useSkillStore.getState().clearSearch()
      expect(useSkillStore.getState().searchResults).toEqual([])
    })
  })

  describe('deleteSkill', () => {
    it('returns true and removes skill from list on 204', async () => {
      useSkillStore.setState({ skills: [mockSkillSummary] })
      mockFetch.mockReturnValue(
        Promise.resolve({ ok: true, status: 204 } as Response)
      )

      let result: unknown
      await act(async () => {
        result = await useSkillStore.getState().deleteSkill('skill-1')
      })

      expect(result).toBe(true)
      expect(useSkillStore.getState().skills).toHaveLength(0)
    })

    it('returns false on error response', async () => {
      mockFetch.mockReturnValue(mockErrorResponse(404))

      let result: unknown
      await act(async () => {
        result = await useSkillStore.getState().deleteSkill('nonexistent')
      })

      expect(result).toBe(false)
    })
  })
})
