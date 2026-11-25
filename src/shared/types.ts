// Shared types between main, preload and renderer. No Node/Electron imports here.

export type SpeedProfile = 'safe' | 'aggressive';

export type SessionStatus =
  | 'pending'
  | 'running'
  | 'paused'
  | 'complete'
  | 'error'
  | 'cancelled';

export type TaskKind = 'entity' | 'child';
export type TaskStatus = 'pending' | 'running' | 'done' | 'error' | 'skipped';

export type FileStatus = 'pending' | 'downloading' | 'done' | 'error' | 'skipped';

export interface SessionConfig {
  /** Entity ids from the catalog (top-level list endpoints). */
  entities: string[];
  /** Child extractor ids (per-parent sub-resources). */
  children: string[];
  /** Download binary files referenced by `url`-style fields. */
  downloadFiles: boolean;
  profile: SpeedProfile;
  /** Root folder the user picked; a timestamped subfolder is created inside. */
  destDir: string;
  /** Display name shown in the extractions list. */
  name: string;
  /** Provider-supplied endpoint specs used by the extractor. */
  entitySpecs?: EntitySpec[];
  childSpecs?: ChildSpec[];
}

export interface ConnectionRow {
  id: string;
  createdAt: string;
  provider: string;
  name: string;
  host: string;
  apiKey: string;
  /** Provider-defined extra fields (e.g. OAuth client_id/secret). */
  credentials?: Record<string, string>;
  lastOk: boolean | null;
}

export interface ConnectionInput {
  id?: string | null;
  provider: string;
  name: string;
  host: string;
  apiKey: string;
  credentials?: Record<string, string>;
}

export interface SessionRow {
  id: string;
  connectionId: string;
  createdAt: string;
  updatedAt: string;
  status: SessionStatus;
  profile: SpeedProfile;
  destDir: string;
  exportDir: string;
  config: SessionConfig;
}

export interface TaskProgress {
  id: number;
  kind: TaskKind;
  entity: string;
  label: string;
  parentRef: string | null;
  status: TaskStatus;
  page: number;
  fetched: number;
  attempts: number;
  lastError: string | null;
}

export interface FileProgress {
  total: number;
  done: number;
  pending: number;
  error: number;
  skipped: number;
  bytesDone: number;
}

export interface SessionSnapshot {
  session: SessionRow;
  tasks: TaskProgress[];
  files: FileProgress;
  recentEvents: SessionEvent[];
}

export interface SessionEvent {
  id: number;
  sessionId: string;
  ts: string;
  level: 'info' | 'warn' | 'error' | 'addon';
  message: string;
}

export interface EntityInfo {
  id: string;
  label: string;
  group: string;
  description?: string;
  /** True when records may carry a downloadable file `url`. */
  downloadable?: boolean;
}

export interface EntitySpec extends EntityInfo {
  path: string;
  /** "GET" (default) or "POST" — POST sends `bodyTemplate` as JSON. */
  method?: string;
  /** Body template; `{page}`/`{per_page}` substituted per request. */
  bodyTemplate?: string;
  /** Dotted path to the rows array (default "data"). */
  dataPath?: string;
}

export interface ChildSpec extends ChildInfo {
  single?: boolean;
  /** Endpoint path; `{id}` is replaced with the parent record id. */
  path: string;
  /** Optional `q` filter expression; `{id}` is replaced with the parent id. */
  qParam?: string;
  /** Extra query params — values may contain `{id}` for the parent record id. */
  params?: [string, string][];
}

export interface ChildInfo {
  id: string;
  label: string;
  parent: string;
  description?: string;
}

export interface Catalog {
  entities: EntityInfo[];
  children: ChildInfo[];
  groups: string[];
}

export interface ValidateResult {
  ok: boolean;
  message: string;
  /** Auth requires a second-factor code before it can proceed. */
  needs2fa?: boolean;
  /** Optional hint (e.g. where the code was sent). */
  otpHint?: string;
}

export interface KeyFoundEvent {
  token: string;
  source: 'network' | 'storage';
}

export interface AppSettings {
  destDir: string;
  profile: SpeedProfile;
}

export interface NewSessionResult {
  sessionId: string;
  exportDir: string;
}

// Tauri command names (invoke) and event names (listen)
export const CMD = {
  settingsGet: 'settings_get',
  settingsSet: 'settings_set',
  chooseDir: 'choose_dir',
  connectionsList: 'connections_list',
  connectionSave: 'connection_save',
  connectionDelete: 'connection_delete',
  connectionValidate: 'connection_validate',
  connectionOpen: 'connection_open',
  connectionCurrent: 'connection_current',
  connectionKeyHelper: 'connection_key_helper',
  catalogList: 'catalog_list',
  sessionCreate: 'session_create',
  sessionStart: 'session_start',
  sessionPause: 'session_pause',
  sessionResume: 'session_resume',
  sessionCancel: 'session_cancel',
  sessionList: 'session_list',
  sessionSnapshot: 'session_snapshot',
  sessionUpdateConfig: 'session_update_config',
  sessionOpenExport: 'session_open_export',
  sessionSaveConfig: 'session_save_config',
  sessionDelete: 'session_delete',
  sessionExportLog: 'session_export_log',
} as const;

export const EVT = {
  sessionUpdate: 'session:update',
  keyFound: 'key:found',
} as const;
