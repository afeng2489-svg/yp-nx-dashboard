import { useEffect, useState } from 'react';
import { packageVersion } from './packageVersion';
import { message } from '@tauri-apps/plugin-dialog';

interface VersionInfo {
  latestVersion: string;
  releaseNotes: string;
  downloadUrl: string;
}

/** GitHub Releases（生产）或本地 version.json（开发） */
const VERSION_SOURCES = [
  '/version.json',
  'https://api.github.com/repos/nexusflow/nexusflow/releases/latest',
];

function compareVersions(current: string, latest: string): boolean {
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

async function fetchVersionInfo(): Promise<VersionInfo | null> {
  for (const url of VERSION_SOURCES) {
    try {
      const response = await fetch(`${url}${url.startsWith('/') ? '?t=' + Date.now() : ''}`, {
        method: 'GET',
        cache: 'no-cache',
        headers: url.includes('github') ? { Accept: 'application/vnd.github+json' } : {},
      });
      if (!response.ok) continue;

      if (url.includes('github')) {
        const release = (await response.json()) as {
          tag_name?: string;
          body?: string;
          html_url?: string;
        };
        const tag = (release.tag_name ?? '').replace(/^v/, '');
        if (!tag) continue;
        return {
          latestVersion: tag,
          releaseNotes: release.body ?? '',
          downloadUrl: release.html_url ?? '',
        };
      }

      return (await response.json()) as VersionInfo;
    } catch {
      continue;
    }
  }
  return null;
}

export function useVersionCheck() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);

  useEffect(() => {
    void (async () => {
      const info = await fetchVersionInfo();
      if (info && compareVersions(packageVersion, info.latestVersion)) {
        setVersionInfo(info);
        setUpdateAvailable(true);
      }
    })();
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
