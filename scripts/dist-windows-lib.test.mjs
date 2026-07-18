import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { sha256File, formatSha256SumsFile, buildMetadata, resolveInstallerFileName } from './dist-windows-lib.mjs';

test('sha256File matches a manually computed digest', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'omg-dist-test-'));
  try {
    const filePath = join(dir, 'sample.bin');
    const content = Buffer.from('open-mouse-gesture-dist-test');
    writeFileSync(filePath, content);

    const expected = createHash('sha256').update(content).digest('hex');
    const actual = await sha256File(filePath);
    assert.equal(actual, expected);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('formatSha256SumsFile produces sha256sum-compatible lines', () => {
  const out = formatSha256SumsFile([
    { name: 'a.exe', sha256: 'aaaa' },
    { name: 'b.exe', sha256: 'bbbb' },
  ]);
  assert.equal(out, 'aaaa  a.exe\nbbbb  b.exe\n');
});

test('buildMetadata includes version, commit, and artifact hashes', () => {
  const meta = buildMetadata({
    version: '1.0.1',
    productName: 'GestureHotkeyApp',
    buildTimestamp: '2026-07-18T00:00:00.000Z',
    gitCommit: 'deadbeef',
    artifacts: [{ name: 'a.exe', sha256: 'aaaa', sizeBytes: 10 }],
  });
  assert.equal(meta.version, '1.0.1');
  assert.equal(meta.gitCommit, 'deadbeef');
  assert.equal(meta.artifacts.length, 1);
  assert.equal(meta.artifacts[0].sha256, 'aaaa');
});

test('buildMetadata falls back to null commit when unavailable', () => {
  const meta = buildMetadata({
    version: '1.0.1',
    productName: 'GestureHotkeyApp',
    buildTimestamp: '2026-07-18T00:00:00.000Z',
    gitCommit: null,
    artifacts: [],
  });
  assert.equal(meta.gitCommit, null);
});

test('resolveInstallerFileName picks the single x64-setup.exe', () => {
  const name = resolveInstallerFileName(['GestureHotkeyApp_0.1.0_x64-setup.exe', 'other.nsis']);
  assert.equal(name, 'GestureHotkeyApp_0.1.0_x64-setup.exe');
});

test('resolveInstallerFileName throws when none found', () => {
  assert.throws(() => resolveInstallerFileName(['foo.txt']), /No NSIS installer/);
});

test('resolveInstallerFileName throws when multiple candidates found', () => {
  assert.throws(
    () => resolveInstallerFileName(['a_x64-setup.exe', 'b_x64-setup.exe']),
    /Multiple NSIS installer candidates/,
  );
});
