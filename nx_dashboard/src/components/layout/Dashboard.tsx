import { AppShell } from '@/components/layout/shells/AppShell';
import { useNexusflowCommands } from '@/hooks/useNexusflowCommands';
import { watchFactoryApprovals } from '@/services/factoryNotifications';
import { useEffect } from 'react';

export function Dashboard() {
  useNexusflowCommands();
  useEffect(() => {
    const unsub = watchFactoryApprovals();
    return unsub;
  }, []);
  return <AppShell />;
}
