import { ProcessMonitor } from '@/components/team/ProcessMonitor';

export default function ProcessMonitorPage({ embedded = false }: { embedded?: boolean }) {
  return <ProcessMonitor embedded={embedded} />;
}
