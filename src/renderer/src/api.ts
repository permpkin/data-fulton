import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  CMD,
  EVT,
  type AppSettings,
  type Catalog,
  type ConnectionInput,
  type ConnectionRow,
  type KeyFoundEvent,
  type NewSessionResult,
  type SessionConfig,
  type SessionRow,
  type SessionSnapshot,
  type SpeedProfile,
  type ValidateResult,
} from '@shared/types';

// Thin typed wrapper over the Tauri command surface.
export const datafulton = {
  // settings
  getSettings: () => invoke<AppSettings>(CMD.settingsGet),
  setSettings: (settings: AppSettings) => invoke<void>(CMD.settingsSet, { settings }),
  chooseDirectory: () => invoke<string | null>(CMD.chooseDir),

  // connections
  listConnections: () => invoke<ConnectionRow[]>(CMD.connectionsList),
  saveConnection: (input: ConnectionInput) =>
    invoke<ConnectionRow>(CMD.connectionSave, { input }),
  deleteConnection: (id: string) => invoke<void>(CMD.connectionDelete, { id }),
  testConnection: (input: ConnectionInput, otp?: string) => invoke<ValidateResult>("connection_test", { input, otp: otp ?? null }),
  validateConnection: (id: string) => invoke<ValidateResult>(CMD.connectionValidate, { id }),
  openConnection: (id: string) => invoke<ValidateResult>(CMD.connectionOpen, { id }),
  currentConnection: () => invoke<ConnectionRow | null>(CMD.connectionCurrent),
  openKeyHelper: (connectionId: string) =>
    invoke<void>(CMD.connectionKeyHelper, { id: connectionId }),
  onKeyFound: (cb: (e: KeyFoundEvent) => void): Promise<UnlistenFn> =>
    listen<KeyFoundEvent>(EVT.keyFound, (e) => cb(e.payload)),

  // catalog
  getCatalog: () => invoke<Catalog>(CMD.catalogList),

  // sessions
  createSession: (connectionId: string, config: SessionConfig) =>
    invoke<NewSessionResult>(CMD.sessionCreate, { connectionId, config }),
  startSession: (id: string) => invoke<void>(CMD.sessionStart, { id }),
  pauseSession: (id: string) => invoke<void>(CMD.sessionPause, { id }),
  restartSession: (id: string) => invoke<void>("session_restart", { id }),
  resumeSession: (id: string) => invoke<void>(CMD.sessionResume, { id }),
  cancelSession: (id: string) => invoke<void>(CMD.sessionCancel, { id }),
  listSessions: (connectionId: string) =>
    invoke<SessionRow[]>(CMD.sessionList, { connectionId }),
  getSnapshot: (id: string) => invoke<SessionSnapshot | null>(CMD.sessionSnapshot, { id }),
  updateSessionConfig: (id: string, profile: SpeedProfile, downloadFiles: boolean) =>
    invoke<void>(CMD.sessionUpdateConfig, { id, profile, downloadFiles }),
  openExportDir: (id: string) => invoke<void>(CMD.sessionOpenExport, { id }),
  saveSessionConfig: (id: string, config: SessionConfig) => invoke<void>(CMD.sessionSaveConfig, { id, config }),
  deleteSession: (id: string) => invoke<void>(CMD.sessionDelete, { id }),
  clearFinished: (connectionId: string) => invoke<number>("session_clear_finished", { connectionId }),
  openConnections: () => invoke<void>("open_connections"),
  exportLog: (id: string, path: string) => invoke<void>(CMD.sessionExportLog, { id, path }),
  onSessionUpdate: (cb: (s: SessionSnapshot) => void): Promise<UnlistenFn> =>
    listen<SessionSnapshot>(EVT.sessionUpdate, (e) => cb(e.payload)),
};
