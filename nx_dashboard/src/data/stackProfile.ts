/** Stack detection from workspace file listing — shared by routing, launch preview, recovery CTAs */

export type StackLanguage = 'rust' | 'typescript' | 'python' | 'go' | 'node' | 'unknown';

export interface StackProfile {
  language: StackLanguage;
  hasExistingCode: boolean;
  projectType: string;
  packageManager?: string;
  buildCmd?: string;
  testCmd?: string;
  lintCmd?: string;
  markerFiles: string[];
}

type FileEntry = { path: string; is_directory: boolean };

function hasFile(files: FileEntry[], ...names: string[]): boolean {
  const set = new Set(files.filter((f) => !f.is_directory).map((f) => f.path.replace(/^\.\//, '')));
  return names.some((n) => set.has(n) || [...set].some((p) => p.endsWith(`/${n}`)));
}

/** Detect stack from workspace browse listing (top-level markers). */
export function detectStackProfile(files: FileEntry[]): StackProfile {
  const markers: string[] = [];

  if (hasFile(files, 'Cargo.toml')) {
    markers.push('Cargo.toml');
    return {
      language: 'rust',
      hasExistingCode: true,
      projectType: 'rust',
      packageManager: 'cargo',
      buildCmd: 'cargo build',
      testCmd: 'cargo test',
      lintCmd: 'cargo clippy',
      markerFiles: markers,
    };
  }

  if (hasFile(files, 'go.mod')) {
    markers.push('go.mod');
    return {
      language: 'go',
      hasExistingCode: true,
      projectType: 'go',
      packageManager: 'go',
      buildCmd: 'go build ./...',
      testCmd: 'go test ./...',
      lintCmd: 'go vet ./...',
      markerFiles: markers,
    };
  }

  if (hasFile(files, 'pyproject.toml', 'requirements.txt', 'setup.py')) {
    if (hasFile(files, 'pyproject.toml')) markers.push('pyproject.toml');
    if (hasFile(files, 'requirements.txt')) markers.push('requirements.txt');
    return {
      language: 'python',
      hasExistingCode: true,
      projectType: 'python',
      packageManager: 'pip',
      buildCmd: undefined,
      testCmd: 'pytest',
      lintCmd: 'ruff check .',
      markerFiles: markers,
    };
  }

  if (hasFile(files, 'package.json')) {
    markers.push('package.json');
    const hasTs = hasFile(files, 'tsconfig.json');
    return {
      language: hasTs ? 'typescript' : 'node',
      hasExistingCode: true,
      projectType: 'node',
      packageManager: 'npm',
      buildCmd: 'npm run build',
      testCmd: 'npm test',
      lintCmd: 'npm run lint',
      markerFiles: markers,
    };
  }

  const meaningful = files.filter(
    (f) =>
      !f.is_directory &&
      !f.path.startsWith('.') &&
      f.path !== '.gitkeep' &&
      !f.path.endsWith('/.DS_Store'),
  );

  return {
    language: 'unknown',
    hasExistingCode: meaningful.length > 0,
    projectType: 'unknown',
    markerFiles: markers,
  };
}

/** Terminal command hint when a Run fails on quality gate */
export function stackFailureTerminalCommand(profile: StackProfile): string | undefined {
  if (profile.testCmd) return profile.testCmd;
  if (profile.buildCmd) return profile.buildCmd;
  return undefined;
}
