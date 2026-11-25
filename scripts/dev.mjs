/**
 * `npm run dev` — run `tauri dev` plus the provider test servers.
 * Each child runs in its own process group so shutdown kills the whole
 * tree (vite, cargo, and the app binary — not just the direct children).
 */
import { spawn } from 'node:child_process';

const procs = [
  spawn('node', ['scripts/test-servers.mjs'], { stdio: 'inherit', detached: true }),
  spawn('npx', ['tauri', 'dev'], { stdio: 'inherit', detached: true }),
];

const killGroup = (p, sig) => {
  if (!p.pid) return;
  try { process.kill(-p.pid, sig); } catch { try { p.kill(sig); } catch {} }
};

let shuttingDown = false;
const shutdown = () => {
  if (shuttingDown) return;
  shuttingDown = true;
  procs.forEach((p) => killGroup(p, 'SIGTERM'));
  // SIGKILL fallback for anything that ignores SIGTERM.
  setTimeout(() => procs.forEach((p) => killGroup(p, 'SIGKILL')), 1200).unref();
};

procs.forEach((p) => p.on('exit', () => shutdown()));
process.on('SIGINT', () => { shutdown(); process.exit(0); });
process.on('SIGTERM', () => { shutdown(); process.exit(0); });
process.on('exit', shutdown);
