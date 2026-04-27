import { useEffect, useState } from 'react';
import { packageVersion } from './packageVersion';
import { message } from '@tauri-apps/plugin-dialog';

interface VersionInfo {
  latestVersion: string;
  releaseNotes: string;
  downloadUrl: string;
}

function compareVersions(current: string, latest: string): boolean {
  // Returns true if current < latest
  const currentParts = current.split('.').map(Number);
  const latestParts = latest.split('.').map(Number);

  for (let i = 0; i < Math.max(currentParts.length, latestParts.length); i++) {
    const c = currentParts[i] || 0;
    const l = latestParts[i] || 0;
    if (c < l) return true;
    if (c > l) return false;
  }
  return false;
}

export function useVersionCheck() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);

  useEffect(() => {
    async function checkVersion() {
      try {
        // Check against local version.json (for development)
        // In production, this would be hosted on your server
        const response = await fetch('/version.json?t=' + Date.now(), {
          method: 'GET',
          cache: 'no-cache',
        });

        if (!response.ok) {
          console.log('Version check skipped: version.json not found');
          return;
        }

        const info: VersionInfo = await response.json();

        if (compareVersions(packageVersion, info.latestVersion)) {
          setVersionInfo(info);
          setUpdateAvailable(true);
        }
      } catch (error) {
        console.log('Version check failed:', error);
      }
    }

    checkVersion();
  }, []);

  const showUpdateDialog = async () => {
    if (!versionInfo) return;

    await message(
      `发现新版本 ${versionInfo.latestVersion}！\n\n${versionInfo.releaseNotes}\n\n当前版本: ${packageVersion}`,
      {
        title: '发现新版本',
        kind: 'info',
        okLabel: '知道了',
      },
    );
  };

  return {
    updateAvailable,
    versionInfo,
    showUpdateDialog,
  };
}
