// Pure helper functions for the repository-level Windows distribution export.
// Kept free of filesystem/process side effects so they can be unit tested directly.
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';

export function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

export function formatSha256SumsFile(entries) {
  // sha256sum-compatible format: "<hash>  <filename>\n"
  return entries.map((e) => `${e.sha256}  ${e.name}`).join('\n') + '\n';
}

export function buildMetadata({ version, productName, buildTimestamp, gitCommit, artifacts }) {
  return {
    productName,
    version,
    buildTimestamp,
    gitCommit: gitCommit || null,
    artifacts: artifacts.map((a) => ({
      name: a.name,
      sha256: a.sha256,
      sizeBytes: a.sizeBytes,
    })),
  };
}

// Picks the single NSIS installer exe from a directory listing, or throws a
// descriptive error if none/multiple are found.
export function resolveInstallerFileName(dirEntries) {
  const matches = dirEntries.filter((name) => /_x64-setup\.exe$/i.test(name));
  if (matches.length === 0) {
    throw new Error('No NSIS installer (*_x64-setup.exe) found in bundle/nsis output.');
  }
  if (matches.length > 1) {
    throw new Error(`Multiple NSIS installer candidates found: ${matches.join(', ')}`);
  }
  return matches[0];
}
