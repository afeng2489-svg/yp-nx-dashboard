import { create } from 'zustand';

interface ContextPanelStore {
  selectedExecutionId: string | null;
  isOpen: boolean;
  selectExecution: (id: string | null) => void;
  close: () => void;
}

export const useContextPanelStore = create<ContextPanelStore>((set) => ({
  selectedExecutionId: null,
  isOpen: false,
  selectExecution: (id) => set({ selectedExecutionId: id, isOpen: id !== null }),
  close: () => set({ selectedExecutionId: null, isOpen: false }),
}));
