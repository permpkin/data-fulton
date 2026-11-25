import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { save } from '@tauri-apps/plugin-dialog';
import { datafulton } from './api';
import type {
  AppSettings,
  Catalog,
  ConnectionRow,
  SessionConfig,
  SessionRow,
  SessionSnapshot,
  SpeedProfile,
  ValidateResult,
} from '@shared/types';
import { Button } from './components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from './components/ui/card';
import { Input } from './components/ui/input';
import { Label } from './components/ui/label';
import { Badge, statusVariant } from './components/ui/badge';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './components/ui/table';
import { Checkbox } from './components/ui/checkbox';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './components/ui/select';
import { PROVIDERS, getProvider } from './providers';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from './components/ui/dropdown-menu';
import { RadioGroup, RadioGroupItem } from './components/ui/radio-group';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from './components/ui/accordion';
import {
  ArrowLeft, Check, ChevronDown, Download, Ellipsis, FolderOpen, LoaderCircle, ScrollText, X, Settings, Pause, Play, Plus, CircleStop, RotateCcw, RefreshCw, Eraser, Trash,
} from 'lucide-react';

const DOT_COLOR: Record<string, string> = {
  running: 'bg-blue-400',
  complete: 'bg-emerald-400',
  paused: 'bg-amber-400',
  pending: 'bg-amber-400',
  error: 'bg-destructive',
  cancelled: 'bg-destructive',
};

const LOG_COLOR: Record<string, string> = {
  warn: 'text-amber-400',
  error: 'text-destructive',
  addon: 'text-purple-400',
  info: 'text-muted-foreground',
};

const DEFAULT_HOST = 'https://www.primeeco.tech/api.prime';
export default function App() {
  return window.location.hash === '#main' ? <MainWindow /> : <ConnectionsWindow />;
}

function Toolbar({
  backLabel = 'Back',
  onBack,
  title,
  children,
}: {
  backLabel?: string;
  onBack: () => void;
  title?: React.ReactNode;
  children?: React.ReactNode;
}) {
  return (
    <motion.div
      initial={{ y: -12, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.18 }}
      className="sticky top-0 z-10 flex items-center border-b border-border/50 bg-background/80 px-3 py-2 backdrop-blur-md"
      data-tauri-drag-region
    >
      <Button variant="ghost" size="sm" onClick={onBack}>
        <ArrowLeft />
        <AnimatePresence mode="wait" initial={false}>
          <motion.span key={backLabel} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.12 }}>
            {backLabel}
          </motion.span>
        </AnimatePresence>
      </Button>
      {title && (
        <span className="absolute left-1/2 -translate-x-1/2 max-w-[45%] truncate text-xs font-medium text-muted-foreground">
          {title}
        </span>
      )}
      {children}
    </motion.div>
  );
}

// ---------------- connections launcher window ----------------

function ConnectionsWindow() {
  const [connections, setConnections] = useState<ConnectionRow[]>([]);
  const [form, setForm] = useState<'new' | ConnectionRow | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const saveRef = useRef<() => void>(() => {});
  const [formBusy, setFormBusy] = useState(false);
  const [activeConns, setActiveConns] = useState<Set<string>>(new Set());
  const [justSaved, setJustSaved] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const cs = await datafulton.listConnections();
    setConnections(cs);
    // Which connections have actively running extractions right now?
    const active = new Set<string>();
    await Promise.all(cs.map(async (c) => {
      const ss = await datafulton.listSessions(c.id);
      if (ss.some((x) => x.status === 'running')) active.add(c.id);
    }));
    setActiveConns(active);
  }, []);
  useEffect(() => void refresh(), [refresh]);

  // Live updates: a session starting/finishing refreshes the running set.
  useEffect(() => {
    let unsub: (() => void) | undefined;
    void datafulton.onSessionUpdate((snap) => {
      if (snap.session.status === 'running') {
        setActiveConns((prev) => new Set(prev).add(snap.session.connectionId));
      } else {
        void datafulton.listSessions(snap.session.connectionId).then((ss) =>
          setActiveConns((prev) => {
            const n = new Set(prev);
            if (ss.some((x) => x.status === 'running')) n.add(snap.session.connectionId);
            else n.delete(snap.session.connectionId);
            return n;
          }),
        );
      }
    }).then((u) => (unsub = u));
    return () => unsub?.();
  }, []);

  const open = async (id: string) => {
    setBusy(id);
    setError(null);
    try {
      const res = await datafulton.openConnection(id); // opens main window + closes this one
      if (!res.ok) setError(res.message);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      {form !== null ? (
        <motion.main className="max-h-screen overflow-y-auto" data-tauri-drag-region
          initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.18 }}>
          <Toolbar onBack={() => setForm(null)} title={form === 'new' ? 'New connection' : 'Connection settings'}>
            <Button variant="outline" className="ml-auto" disabled={formBusy} onClick={() => saveRef.current()}>
              {formBusy ? 'Working…' : 'Save'}
            </Button>
          </Toolbar>
          <div className="mx-auto flex w-full max-w-md flex-col gap-4 p-5">
            <ConnectionForm
              conn={form === 'new' ? undefined : form}
              saveRef={saveRef}
              onBusy={setFormBusy}
              onSaved={async (c) => {
                await refresh();
                setForm(null);
                setJustSaved(c.id);
                setTimeout(() => setJustSaved(null), 1600);
                if (form === 'new') void open(c.id); // straight into extractions on create
              }}
              onDeleted={async () => { setForm(null); await refresh(); }}
            />
          </div>
        </motion.main>
      ) : (
      <motion.main className="flex min-h-screen items-center justify-center p-5" data-tauri-drag-region
        initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.2 }}>
        <div className="flex w-full max-w-md flex-col gap-4">
          <div>
            <h2 className="text-base font-semibold">Connections</h2>
          </div>

          {error && (
            <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs text-destructive">
              {error}
            </div>
          )}

          {connections.length === 0 && (
            <p className="text-xs text-muted-foreground">No connections yet — add your first one.</p>
          )}

          {connections.map((c, i) => (
            <motion.div key={c.id} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.05, duration: 0.2 }}>
            <Card className={`overflow-hidden py-0 transition-shadow duration-700 ${justSaved === c.id ? 'ring-2 ring-emerald-500/70 shadow-[0_0_18px_rgba(52,211,153,0.4)]' : ''}`}>
              <div className="flex items-stretch">
                <button
                  className="flex min-w-0 flex-1 cursor-pointer items-center gap-3 px-6 py-4 text-left transition-colors hover:bg-accent disabled:opacity-50"
                  disabled={busy !== null}
                  onClick={() => open(c.id)}
                >
                  {activeConns.has(c.id) && (
                    <LoaderCircle className="size-3.5 shrink-0 animate-spin text-blue-500" />
                  )}
                  <span className="min-w-0 flex-1">
                    <span className="block text-xs font-medium">
                      {busy === c.id ? 'Connecting…' : c.name}
                    </span>
                    <span className="block truncate text-xs text-muted-foreground">{c.host}</span>
                  </span>
                </button>
                <div className="w-px bg-border" />
                <button
                  className="flex cursor-pointer items-center px-4 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                  title="Edit"
                  onClick={() => setForm(c)}
                >
                  <Settings className="size-4" />
                </button>
              </div>
            </Card>
            </motion.div>
          ))}

          <Button variant="outline" className="w-full" onClick={() => setForm('new')}>
            <Plus />New connection
          </Button>
        </div>
      </motion.main>
      )}
    </>
  );
}

// ---------------- main extractions window ----------------

function MainWindow() {
  const [conn, setConn] = useState<ConnectionRow | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<SessionRow | null>(null);
  const [formStep, setFormStep] = useState(1);
  const [sessions, setSessions] = useState<SessionRow[]>([]);
  const [justSaved, setJustSaved] = useState<string | null>(null);
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [bar, setBar] = useState<{
    title: string;
    backLabel: string;
    back: () => void;
    next?: () => void;
    nextLabel: string;
    nextDisabled: boolean;
  } | null>(null);

  useEffect(() => {
    void datafulton.currentConnection().then(setConn);
    void datafulton.currentConnection().then((c) => { if (c) void getProvider(c.provider || 'primeeco').catalog().then(setCatalog); });
  }, []);

  const refreshSessions = useCallback(() => {
    if (conn) void datafulton.listSessions(conn.id).then(setSessions);
  }, [conn?.id]);

  const [refreshing, setRefreshing] = useState(false);
  const [progress, setProgress] = useState<Record<string, { done: number; errors: number; total: number; pct: number; segs: Record<string, number> }>>({});

  const loadSessions = useCallback((ss: SessionRow[]) => {
    setSessions(ss);
    ss.forEach((x) =>
      void datafulton.getSnapshot(x.id).then((snap) => {
        if (!snap) return;
        const done = snap.tasks.filter((t) => t.status === 'done').length + snap.files.done + snap.files.skipped;
        const errors = snap.tasks.filter((t) => t.status === 'error').length + snap.files.error;
        const total = snap.tasks.length + snap.files.total;
        const pct = total ? Math.round(((done + errors) / total) * 100) : 0;
        setProgress((prev) => {
          // Don't let a transient empty read wipe a known non-zero total.
          if (total === 0 && (prev[x.id]?.total ?? 0) > 0) return prev;
          return { ...prev, [x.id]: { done, errors, total, pct, segs: taskSegs(snap) } };
        });
      }),
    );
  }, []);

  const reload = useCallback(() => {
    if (!conn) return;
    setRefreshing(true);
    void datafulton.listSessions(conn.id).then((ss) => { loadSessions(ss); setRefreshing(false); });
  }, [conn?.id]);

  useEffect(() => {
    if (!conn) return;
    void datafulton.listSessions(conn.id).then(loadSessions);
    let unsub: (() => void) | undefined;
    void datafulton.onSessionUpdate((snap) => {
      if (snap.session.connectionId !== conn.id) return;
      setSessions((prev) => {
        const idx = prev.findIndex((x) => x.id === snap.session.id);
        if (idx === -1) {
          // Unknown session (e.g. just created) — refetch the list once.
          void datafulton.listSessions(conn.id).then(loadSessions);
          return prev;
        }
        const next = [...prev];
        next[idx] = snap.session;
        return next;
      });
      const done = snap.tasks.filter((t) => t.status === 'done').length + snap.files.done + snap.files.skipped;
      const errors = snap.tasks.filter((t) => t.status === 'error').length + snap.files.error;
      const total = snap.tasks.length + snap.files.total;
      const pct = total ? Math.round(((done + errors) / total) * 100) : 0;
      setProgress((prev) => {
        if (total === 0 && (prev[snap.session.id]?.total ?? 0) > 0) return prev;
        return { ...prev, [snap.session.id]: { done, errors, total, pct, segs: taskSegs(snap) } };
      });
    }).then((u) => (unsub = u));
    return () => unsub?.();
  }, [conn?.id]);

  if (!conn || !catalog) {
    return <div className="p-20 text-center text-muted-foreground"><p>Loading…</p></div>;
  }

  const showForm = !sessionId && creating;

  const viewKey = sessionId ? `run-${sessionId}` : showForm ? 'form' : 'list';
  return (
    <main className="max-h-screen overflow-y-auto" data-tauri-drag-region>
      <AnimatePresence mode="wait" initial={false}>
      <motion.div key={viewKey} initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 1.04 }} transition={{ duration: 0.18 }}>
      {sessionId ? (
        <RunScreen sessionId={sessionId} onBack={() => { setSessionId(null); reload(); }} />
      ) : showForm ? (
        <>
          <Toolbar
            backLabel={bar?.backLabel ?? 'Back'}
            onBack={() => (bar?.back ? bar.back() : setCreating(false))}
            title={bar?.title}
          >
            <AnimatePresence mode="wait" initial={false}>
            {bar?.next ? (
              <motion.div key={bar.nextLabel} className="ml-auto"
                initial={{ opacity: 0, x: 10 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 10 }}
                transition={{ duration: 0.15 }}>
                <Button
                  variant="outline"
                  disabled={bar.nextDisabled}
                  onClick={bar.next}
                >
                  {bar.nextLabel}
                </Button>
              </motion.div>
            ) : (
              <motion.span key="empty" className="ml-auto" />
            )}
            </AnimatePresence>
          </Toolbar>
          <div className="mx-auto flex w-full max-w-md flex-col gap-4 p-5">
            <NewSessionForm
              conn={conn}
              catalog={catalog}
              session={editing ?? undefined}
              initialStep={formStep}
              defaultName={`Untitled extraction ${sessions.length + 1}`}
              onBar={setBar}
              onExit={(sid) => { setCreating(false); setEditing(null); setBar(null); setFormStep(1); if (sid) { setJustSaved(sid); setTimeout(() => setJustSaved(null), 1600); } reload(); }}
              onStarted={(id) => { setCreating(false); setEditing(null); setFormStep(1); setBar(null); setSessionId(id); }}
            />
          </div>
        </>
      ) : (
        <div className="relative flex min-h-screen flex-col">
          <Toolbar
            onBack={() => void datafulton.openConnections()}
            backLabel="Connections"
            title={conn.name}
          >
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="ml-auto"><Ellipsis /></Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={reload}><RefreshCw />Refresh</DropdownMenuItem>
                <DropdownMenuItem
                  disabled={!sessions.some((x) => ['complete', 'error', 'cancelled'].includes(x.status))}
                  onClick={async () => { await datafulton.clearFinished(conn.id); reload(); }}
                >
                  <Eraser />Clear all
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </Toolbar>
          <div className="flex flex-1 items-center justify-center p-5">
          {refreshing && (
            <div className="absolute inset-0 z-20 flex items-center justify-center bg-background/70 backdrop-blur-sm">
              <LoaderCircle className="size-6 animate-spin text-blue-500" />
            </div>
          )}
          <div className="flex w-full max-w-md flex-col gap-4">

            {sessions.length === 0 && (
              <p className="text-xs text-muted-foreground">
                No extractions yet — click <b>+ New extraction</b> below to choose what to pull
                from your account.
              </p>
            )}

            {['running', 'paused', 'pending', 'error', 'complete', 'cancelled']
              .map((st) => sessions.filter((s) => s.status === st))
              .filter((g) => g.length > 0)
              .map((group) => (
                <div key={group[0].status} className="flex flex-col gap-2">
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    {group[0].status}
                  </h3>
                  {group.map((s, i) => (
                    <motion.div key={s.id} layout initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, scale: 0.98 }} transition={{ delay: i * 0.04, duration: 0.2 }}>
                    <Card className={`overflow-hidden py-0 transition-shadow duration-700 ${justSaved === s.id ? 'ring-2 ring-emerald-500/70 shadow-[0_0_18px_rgba(52,211,153,0.4)]' : ''}`}>
                      <button
                        className="flex w-full min-w-0 cursor-pointer items-center gap-3 px-6 py-4 text-left transition-colors hover:bg-accent"
                        onClick={() => {
                          if (s.status === 'pending') { setEditing(s); setCreating(true); }
                          else setSessionId(s.id);
                        }}
                      >
                        {s.status === 'running' && (
                          <LoaderCircle className="size-3.5 shrink-0 animate-spin text-blue-500" />
                        )}
                        <span className="min-w-0 flex-1">
                          <span className="block text-xs font-medium">
                            {s.config.name || s.createdAt.slice(0, 16).replace('T', ' ')}
                          </span>
                          <span className="block truncate text-xs text-muted-foreground">
                            {progress[s.id]
                              ? `${progress[s.id].done} done · ${progress[s.id].errors} errors · ${progress[s.id].total} total`
                              : s.exportDir}
                          </span>
                          {s.status === 'running' && progress[s.id] && (
                            <SegmentedBar className="mt-2 h-1.5" segs={progress[s.id].segs} />
                          )}
                        </span>
                        <Badge variant={statusVariant(s.status)}>
                          {s.status === 'paused' && progress[s.id] ? `paused ${progress[s.id].pct}%` : s.status}
                        </Badge>
                      </button>
                    </Card>
                    </motion.div>
                  ))}
                </div>
              ))}

            <Button variant="outline" className="w-full" onClick={() => setCreating(true)}>
              <Plus />New extraction
            </Button>
          </div>
          </div>
        </div>
      )}
      </motion.div>
      </AnimatePresence>
    </main>
  );
}

// ---------------- connection form (new + edit) ----------------

function ConnectionForm({
  conn,
  onSaved,
  onDeleted,
  saveRef,
  onBusy,
}: {
  conn?: ConnectionRow;
  onSaved: (c: ConnectionRow) => void | Promise<void>;
  onDeleted?: () => void | Promise<void>;
  saveRef: React.MutableRefObject<() => void>;
  onBusy: (b: boolean) => void;
}) {
  const [provider, setProvider] = useState(conn?.provider ?? '');
  const [name, setName] = useState(conn?.name ?? '');
  const [host, setHost] = useState(conn?.host ?? getProvider(conn?.provider ?? 'primeeco').defaultHost);
  const [apiKey, setApiKey] = useState(conn?.apiKey ?? '');
  const [status, setStatus] = useState<ValidateResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [creds, setCreds] = useState<Record<string, string>>(conn?.credentials ?? {});

  const fieldValue = (k: string) =>
    k === 'host' ? host : k === 'apiKey' ? apiKey : k === 'name' ? name : (creds[k] ?? '');
  const setField = (k: string, v: string) => {
    if (k === 'host') setHost(v);
    else if (k === 'apiKey') setApiKey(v);
    else if (k === 'name') setName(v);
    else setCreds((prev) => ({ ...prev, [k]: v }));
    setStatus(null);
  };
  const fieldErrs = (includeName: boolean) => {
    const errs: Record<string, string> = {};
    if (!provider) errs.platform = 'Choose a platform';
    if (includeName && !name.trim()) errs.name = 'Name is required';
    if (provider) {
      for (const f of getProvider(provider).fields) {
        if (!fieldValue(f.key).trim()) errs[f.key] = `${f.label} is required`;
      }
    }
    return errs;
  };

  const save = async (nav = true): Promise<ConnectionRow | null> => {
    const errs = fieldErrs(true);
    setFieldErrors(errs);
    if (Object.keys(errs).length) return null;
    setBusy(true);
    try {
      const c = await datafulton.saveConnection({ provider, id: conn?.id ?? null, name, host, apiKey, credentials: creds });
      if (nav) await onSaved(c);
      return c;
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
      return null;
    } finally {
      setBusy(false);
    }
  };

  const [testing, setTesting] = useState(false);
  const [otp, setOtp] = useState('');
  const test = async () => {
    const errs = fieldErrs(false);
    setFieldErrors(errs);
    if (Object.keys(errs).length) return;
    setTesting(true);
    setStatus(null);
    try {
      setStatus(await datafulton.testConnection({ provider, id: null, name: '', host, apiKey, credentials: creds }, otp || undefined));
    } finally {
      setTesting(false);
    }
  };

  // Expose the save action + busy/status to the sticky top bar.
  useEffect(() => {
    saveRef.current = save;
  });
  useEffect(() => onBusy(busy), [busy]);

  return (
    <>
      <AnimatePresence>
        {status && !status.ok && !status.needs2fa && (
          <motion.div
            initial={{ opacity: 0, y: -16, scale: 0.97 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -16, scale: 0.97 }}
            transition={{ duration: 0.18 }}
            className="fixed left-1/2 top-14 z-50 w-[min(400px,88vw)] -translate-x-1/2 rounded-lg border border-destructive/50 bg-destructive/15 px-3 py-2.5 shadow-lg backdrop-blur"
          >
            <div className="flex items-start gap-2.5">
              <p className="flex-1 text-xs leading-relaxed text-destructive">{status.message}</p>
              <button onClick={() => setStatus(null)} className="shrink-0 text-destructive/70 hover:text-destructive" aria-label="Dismiss">
                <X className="size-3.5" />
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
      <div className="flex flex-col gap-4">
        <div className="grid gap-1.5">
          <Label htmlFor="conn-name">Name</Label>
          <Input id="conn-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="My account" aria-invalid={!!fieldErrors.name} />
          {fieldErrors.name && <p className="text-xs text-destructive">{fieldErrors.name}</p>}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="conn-platform">Platform</Label>
          <Select
            value={provider}
            onValueChange={(v) => {
              setProvider(v);
              setHost(getProvider(v).defaultHost);
              if (!name.trim()) setName(`Untitled ${getProvider(v).name}`);
              setStatus(null);
            }}
          >
            <SelectTrigger id="conn-platform" aria-invalid={!!fieldErrors.platform}><SelectValue placeholder="- - select platform - -" /></SelectTrigger>
            <SelectContent>
              {PROVIDERS.map((p) => (
                <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
              ))}
              <SelectItem value="__soon" disabled>More coming soon…</SelectItem>
            </SelectContent>
          </Select>
          {fieldErrors.platform && <p className="text-xs text-destructive">{fieldErrors.platform}</p>}
        </div>
        {provider && getProvider(provider).fields.map((f) => (
          <div key={f.key} className="grid gap-1.5">
            <Label htmlFor={`conn-${f.key}`}>{f.label}</Label>
            <Input
              id={`conn-${f.key}`}
              type={f.password ? 'password' : 'text'}
              value={fieldValue(f.key)}
              onChange={(e) => setField(f.key, e.target.value)}
              placeholder={f.placeholder}
              aria-invalid={!!fieldErrors[f.key]}
            />
            {fieldErrors[f.key] && <p className="text-xs text-destructive">{fieldErrors[f.key]}</p>}
            {f.instructions && (
              <details className="text-xs">
                <summary className="cursor-pointer text-muted-foreground">{f.instructions.title}</summary>
                <ol className="mt-2 ml-5 list-decimal space-y-1 text-muted-foreground">
                  {f.instructions.steps.map((step) => (
                    <li key={step}>{step}</li>
                  ))}
                </ol>
              </details>
            )}
          </div>
        ))}

        {status?.needs2fa && (
          <div className="grid gap-1.5">
            <Label htmlFor="conn-otp">Two-factor code</Label>
            <Input
              id="conn-otp"
              value={otp}
              onChange={(e) => { setOtp(e.target.value); setStatus(null); }}
              placeholder="Enter the 6-digit code"
            />
            {status.otpHint && <p className="text-xs text-muted-foreground">{status.otpHint}</p>}
          </div>
        )}
        <Button
          variant="outline"
          className={`w-full transition-shadow ${
            status
              ? status.ok
                ? 'ring-2 ring-emerald-500/70 shadow-[0_0_16px_rgba(52,211,153,0.35)]'
                : 'ring-2 ring-destructive/70 shadow-[0_0_16px_rgba(239,68,68,0.3)]'
              : ''
          }`}
          disabled={busy || testing}
          onClick={() => void test()}
        >
          {testing && <LoaderCircle className="animate-spin" />}
          Test connection
          <AnimatePresence mode="wait" initial={false}>
            {!testing && status && (
              <motion.span
                key={String(status.ok)}
                initial={{ opacity: 0, scale: 0.6 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.15 }}
                className="flex min-w-0 items-center gap-1"
              >
                {status.ok && <Check className="size-4 text-emerald-400" />}
              </motion.span>
            )}
          </AnimatePresence>
        </Button>
        {conn && onDeleted && (
          <Button
            variant="destructive"
            className="w-full"
            onClick={async () => {
              if (confirm('Delete this connection and its extraction history? (Exported files stay on disk.)')) {
                await datafulton.deleteConnection(conn.id);
                await onDeleted();
              }
            }}
          >
            <Trash size={16} />Delete
          </Button>
        )}
      </div>
    </>
  );
}

// ---------------- sessions pane (list + new) ----------------

type BarSpec = {
  title: string;
  backLabel: string;
  back: () => void;
  next?: () => void;
  nextLabel: string;
  nextDisabled: boolean;
};

function NewSessionForm({
  conn,
  catalog,
  session,
  initialStep = 1,
  defaultName,
  onBar,
  onExit,
  onStarted,
}: {
  conn: ConnectionRow;
  catalog: Catalog;
  session?: SessionRow;
  initialStep?: number;
  defaultName: string;
  onBar: (b: BarSpec | null) => void;
  onExit: (savedSessionId?: string) => void;
  onStarted: (id: string) => void;
}) {
  const [step, setStep] = useState(initialStep);
  const [name, setName] = useState(session?.config.name || defaultName);
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(session ? session.config.entities.filter((e) => e !== 'attachments') : []),
  );
  const [children, setChildren] = useState<Set<string>>(new Set(session?.config.children ?? []));
  const [downloadFiles, setDownloadFiles] = useState(session?.config.downloadFiles ?? true);
  const [settings, setSettings] = useState<AppSettings>({
    destDir: session?.config.destDir ?? '',
    profile: session?.config.profile ?? 'safe',
  });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void datafulton.getSettings().then(setSettings);
  }, []);

  const hasChildParent = (c: { parent: string }) => selected.has(c.parent);
  const anyChildParent = catalog.children.some(hasChildParent);
  const toggle = (set_: Set<string>, id: string, setter: (s: Set<string>) => void) => {
    const next = new Set(set_);
    if (next.has(id)) next.delete(id); else next.add(id);
    setter(next);
  };

  const grouped = useMemo(() => {
    const m = new Map<string, typeof catalog.entities>();
    for (const g of catalog.groups) m.set(g, []);
    for (const e of catalog.entities) m.get(e.group)!.push(e);
    return m;
  }, [catalog]);

  const buildConfig = (): SessionConfig => {
    const entities = new Set(selected);
    if (downloadFiles) entities.add('attachments');
    const prov = getProvider(conn.provider || 'primeeco');
    return {
      entities: [...entities],
      children: [...children].filter((cid) => hasChildParent(catalog.children.find((c) => c.id === cid) ?? { parent: '' })),
      downloadFiles,
      profile: settings.profile,
      destDir: settings.destDir,
      name: name.trim() || defaultName,
      entitySpecs: prov.entitySpecs.filter((e) => entities.has(e.id)),
      childSpecs: prov.childSpecs.filter((c) => children.has(c.id) && hasChildParent(c)),
    };
  };

  const saveLater = async () => {
    setBusy(true);
    setError(null);
    try {
      await datafulton.setSettings(settings);
      const sid = session?.id ?? (await datafulton.createSession(conn.id, buildConfig())).sessionId;
      if (session) await datafulton.saveSessionConfig(session.id, buildConfig());
      onExit(sid);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      await datafulton.setSettings(settings);
      const sessionId = session?.id ?? (await datafulton.createSession(conn.id, buildConfig())).sessionId;
      if (session) await datafulton.saveSessionConfig(sessionId, buildConfig());
      await datafulton.startSession(sessionId);
      onStarted(sessionId);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  // Drive the sticky bar from step state.
  useEffect(() => {
    if (step === 1) {
      onBar({
        title: 'Field selection',
        backLabel: 'Cancel',
        back: onExit,
        next: () => setStep(2),
        nextLabel: 'Next →',
        nextDisabled: selected.size === 0,
      });
    } else if (step === 2) {
      onBar({
        title: 'Settings',
        backLabel: 'Back',
        back: () => setStep(1),
        next: () => setStep(3),
        nextLabel: 'Next →',
        nextDisabled: !settings.destDir,
      });
    } else if (step === 3) {
      onBar({
        title: 'Finalize',
        backLabel: 'Back',
        back: () => setStep(2),
        nextLabel: '',
        nextDisabled: true,
      });
    }
  }, [step, selected.size, settings.destDir, busy]);

  const countLabel = (n: number) => (
    <span className="ml-auto mr-1 text-xs font-normal text-muted-foreground">
      {n > 0 ? `${n} selected` : ''}
    </span>
  );

  if (step === 3) {
    return (
      <motion.div key="step3" initial={{ opacity: 0, x: 16 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.18 }} className="flex flex-col gap-3">
        <div>
          <h2 className="text-base font-semibold">Finalize</h2>
          <p className="text-xs text-muted-foreground">Confirm before starting.</p>
        </div>
        <Card>
          <CardContent className="flex flex-col gap-2 text-xs py-4">
            <div className="flex justify-between"><span className="text-muted-foreground">Name</span><span>{name || defaultName}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Data types</span><span>{selected.size} selected</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Per-record details</span><span>{children.size} selected</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Attached files</span><span>{downloadFiles ? 'yes' : 'no'}</span></div>
            <div className="flex justify-between"><span className="text-muted-foreground">Speed</span><span>{settings.profile}</span></div>
            <div className="flex justify-between gap-3"><span className="text-muted-foreground shrink-0">Destination</span><span className="truncate">{settings.destDir}</span></div>
          </CardContent>
        </Card>
        <Button className="w-full" disabled={busy} onClick={() => void start()}>
          {busy ? 'Starting…' : 'Start extraction'}
        </Button>
        <Button variant="outline" className="w-full" disabled={busy} onClick={() => void saveLater()}>
          Save for later
        </Button>
        {error && (
          <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs text-destructive">{error}</div>
        )}
      </motion.div>
    );
  }

  if (step === 2) {
    return (
      <motion.div key="step2" initial={{ opacity: 0, x: 16 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.18 }} className="flex flex-col gap-4">
        <div>
          <h2 className="text-base font-semibold">Settings</h2>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="ex-name" className="text-xs font-semibold">Name</Label>
          <p className="text-xs text-muted-foreground">Shown in the extractions list.</p>
          <Input id="ex-name" value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div className="flex flex-col gap-1.5 border-t border-border pt-4">
          <Label className="text-xs font-semibold">Files</Label>
          <p className="text-xs text-muted-foreground">Download attached files via each record's <code>url</code> field.</p>
          <label className="flex items-center gap-2.5 cursor-pointer text-xs">
            <Checkbox checked={downloadFiles} onCheckedChange={(v) => setDownloadFiles(v === true)} />
            Download attached files
          </label>
        </div>
        <div className="flex flex-col gap-1.5 border-t border-border pt-4">
          <Label className="text-xs font-semibold">Destination</Label>
          <p className="text-xs text-muted-foreground">Where exported data and files are saved.</p>
          <div className="flex items-center gap-3">
            <Button variant="secondary" size="sm" onClick={async () => {
              const dir = await datafulton.chooseDirectory();
              if (dir) setSettings((s) => ({ ...s, destDir: dir }));
            }}><FolderOpen />Choose folder…</Button>
            <span className="text-xs text-muted-foreground truncate">{settings.destDir || 'No folder chosen'}</span>
          </div>
        </div>
        <div className="flex flex-col gap-1.5 border-t border-border pt-4">
          <Label className="text-xs font-semibold">Speed</Label>
          <p className="text-xs text-muted-foreground">How fast to pull — the app auto-backs-off on rate limits.</p>
        </div>
        <RadioGroup
          value={settings.profile}
          onValueChange={(v) => setSettings((s) => ({ ...s, profile: v as SpeedProfile }))}
          className="grid grid-cols-2 gap-3"
        >
          {([
            { v: 'safe', t: 'Safe', d: '~25 req/min, 1 at a time' },
            { v: 'aggressive', t: 'Aggressive', d: '~150 req/min, 4 at once' },
          ] as const).map(({ v, t, d }) => (
            <Label
              key={v}
              className={`flex items-start gap-3 rounded-lg border p-3 cursor-pointer text-foreground ${
                settings.profile === v ? 'border-primary bg-primary/10' : 'border-border'
              }`}
            >
              <RadioGroupItem value={v} className="mt-0.5" />
              <span>
                <span className="block font-semibold text-xs">{t}</span>
                <span className="block text-xs text-muted-foreground">{d}</span>
              </span>
            </Label>
          ))}
        </RadioGroup>
        {error && (
          <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs text-destructive">{error}</div>
        )}
      </motion.div>
    );
  }

  // Step 1 — field selection
  return (
    <motion.div key="step1" initial={{ opacity: 0, x: -16 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.18 }} className="flex flex-col gap-4">
      <div>
        <h2 className="text-base font-semibold">Field selection</h2>
        <p className="text-xs text-muted-foreground">Choose what to pull from your account.</p>
      </div>
      <label className="flex items-center gap-3 border-b py-4 text-sm font-medium cursor-pointer">
        <Checkbox
          checked={catalog.entities.every((e) => selected.has(e.id)) ? true : catalog.entities.some((e) => selected.has(e.id)) ? 'indeterminate' : false}
          onCheckedChange={(v) => setSelected(v ? new Set(catalog.entities.map((e) => e.id)) : new Set())}
        />
        <span className="flex-1">Select all</span>
        {countLabel(selected.size)}
      </label>
      <Accordion type="single" collapsible className="w-full">
        {[...grouped.entries()].map(([group, entities]) => {
          const count = entities.filter((e) => selected.has(e.id)).length;
          return (
            <AccordionItem key={group} value={group}>
              <div className="flex items-center gap-3 [&>h3]:flex-1">
                <Checkbox
                  checked={entities.every((e) => selected.has(e.id)) ? true : count > 0 ? 'indeterminate' : false}
                  onCheckedChange={(v) => setSelected((prev) => { const n = new Set(prev); entities.forEach((e) => (v ? n.add(e.id) : n.delete(e.id))); return n; })}
                />
                <AccordionTrigger className="flex-1"><span className="flex-1">{group}</span>{countLabel(count)}</AccordionTrigger>
              </div>
              <AccordionContent>
                <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                  {entities.map((e) => (
                    <label key={e.id} className="flex items-center gap-2.5 cursor-pointer text-xs">
                      <Checkbox checked={selected.has(e.id)} onCheckedChange={() => toggle(selected, e.id, setSelected)} />
                      {e.label}
                    </label>
                  ))}
                </div>
              </AccordionContent>
            </AccordionItem>
          );
        })}
        <AccordionItem value="per-job">
          <AccordionTrigger><span className="flex-1">Per-record details</span>{countLabel(children.size)}</AccordionTrigger>
          <AccordionContent>
            <p className="mb-2 text-xs text-muted-foreground">One request per parent record — slower. Requires the parent to be selected.</p>
            <div className="grid grid-cols-2 gap-x-4 gap-y-2">
              {catalog.children.map((c) => (
                <label key={c.id} className={`flex items-center gap-2.5 text-xs ${hasChildParent(c) ? 'cursor-pointer' : 'opacity-40'}`}>
                  <Checkbox disabled={!hasChildParent(c)} checked={hasChildParent(c) && children.has(c.id)} onCheckedChange={() => toggle(children, c.id, setChildren)} />
                  {c.label}
                </label>
              ))}
            </div>
          </AccordionContent>
        </AccordionItem>
      </Accordion>
      {error && (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs text-destructive">{error}</div>
      )}
    </motion.div>
  );
}

// ---------------- run/progress view ----------------

const SEG_COLORS: Record<string, string> = {
  done: 'bg-emerald-500',
  running: 'bg-blue-500',
  pending: 'bg-muted-foreground/30',
  error: 'bg-red-500',
  skipped: 'bg-muted-foreground/60',
};

const taskSegs = (snap: { tasks: { status: string }[]; files: { done: number; error: number; pending: number; skipped: number } }) => {
  const segs: Record<string, number> = { done: 0, running: 0, pending: 0, error: 0, skipped: 0 };
  snap.tasks.forEach((t) => { segs[t.status] = (segs[t.status] ?? 0) + 1; });
  segs.done += snap.files.done;
  segs.error += snap.files.error;
  segs.pending += snap.files.pending;
  segs.skipped += snap.files.skipped;
  return segs;
};

function SegmentedBar({ segs, className = 'h-2' }: { segs: Record<string, number>; className?: string }) {
  const total = Object.values(segs).reduce((a, n) => a + n, 0) || 1;
  return (
    <div className={`flex w-full overflow-hidden rounded-full bg-muted ${className}`}>
      {Object.entries(segs).filter(([, n]) => n > 0).map(([k, n]) => (
        <div
          key={k}
          className={`h-full transition-all ${SEG_COLORS[k] ?? 'bg-muted-foreground'}`}
          style={{ width: `${(n / total) * 100}%` }}
        />
      ))}
    </div>
  );
}


function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(2)} GB`;
}

function RunScreen({ sessionId, onBack }: { sessionId: string; onBack: () => void }) {
  const [snap, setSnap] = useState<SessionSnapshot | null>(null);
  const [openCard, setOpenCard] = useState<'entities' | 'log'>('entities');
  const [showLogs, setShowLogs] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let mounted = true;
    const refresh = async () => {
      const s = await datafulton.getSnapshot(sessionId);
      if (mounted && s) setSnap(s);
    };
    void refresh();
    let unsub: (() => void) | undefined;
    void datafulton.onSessionUpdate((s) => { if (mounted && s.session.id === sessionId) setSnap(s); }).then((u) => (unsub = u));
    return () => { mounted = false; unsub?.(); };
  }, [sessionId]);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [snap?.recentEvents.length]);

  if (!snap) return <Card><CardContent>Loading…</CardContent></Card>;
  const { session, tasks, files, recentEvents } = snap;

  const entityTasks = tasks.filter((t) => t.kind === 'entity');
  const childTasks = tasks.filter((t) => t.kind === 'child');
  const childDone = childTasks.filter((t) => t.status === 'done').length;
  const totalRecords = entityTasks.reduce((a, t) => a + t.fetched, 0);
  const doneEntities = entityTasks.filter((t) => t.status === 'done').length;
  const total = entityTasks.length + childTasks.length + files.total;
  const done = doneEntities + childDone + files.done + files.skipped;
  const errors = tasks.filter((t) => t.status === 'error').length + files.error;
  const pct = total === 0 ? 0 : Math.round(((done + errors) / total) * 100);

  const running = session.status === 'running';
  const resumable = ['paused', 'error', 'pending', 'cancelled'].includes(session.status);

  return (
    <AnimatePresence mode="wait" initial={false}>
    {showLogs ? (
      <motion.div key="logs" initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 1.04 }} transition={{ duration: 0.18 }}>
        <Toolbar onBack={() => setShowLogs(false)} title="Log">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" title="More" className="ml-auto"><Ellipsis /></Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={async () => {
                  const path = await save({
                    defaultPath: `${(session.config.name || 'extraction').replace(/[^\w-]+/g, '_')}-log.txt`,
                  });
                  if (path) await datafulton.exportLog(sessionId, path);
                }}
              >
                <Download />Download log
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </Toolbar>
        <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 px-5 py-5">
          <div
            className="h-[70vh] overflow-y-auto rounded-lg border border-border bg-muted/40 px-3 py-2.5 font-mono text-[11px] leading-relaxed"
            ref={logRef}
          >
            {recentEvents.map((e) => (
              <div key={e.id} className={LOG_COLOR[e.level] ?? 'text-muted-foreground'}>
                {e.ts.slice(11, 19)} {e.message}
              </div>
            ))}
          </div>
        </div>
      </motion.div>
    ) : (
      <motion.div key="progress" initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 1.04 }} transition={{ duration: 0.18 }}>

        <Toolbar onBack={onBack} title={session.config.name || 'Extraction'}>
        <div className="ml-auto flex items-center gap-1">
          {running && (
            <Button variant="ghost" size="icon" title="Pause" onClick={() => datafulton.pauseSession(sessionId)}><Pause /></Button>
          )}
          {resumable && (
            <Button
              variant="ghost"
              size="icon"
              title={session.status === 'pending' ? 'Start' : session.status === 'error' || session.status === 'cancelled' ? 'Retry' : 'Resume'}
              onClick={() => {
                // Optimistic clear: restart wipes shown errors; they reappear only if they reoccur.
                if (session.status === 'error' || session.status === 'cancelled') {
                  setSnap((prev) =>
                    prev
                      ? {
                          ...prev,
                          tasks: prev.tasks.map((t) => ({ ...t, lastError: null })),
                          files: { ...prev.files, error: 0 },
                        }
                      : prev,
                  );
                }
                void datafulton.resumeSession(sessionId);
              }}
            >
              {session.status === 'cancelled' || session.status === 'error' ? <RotateCcw /> : <Play />}
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" title="More"><Ellipsis /></Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => setShowLogs(true)}>
                <ScrollText />View logs
              </DropdownMenuItem>
              {(session.status === 'error' || session.status === 'cancelled') && (
                <DropdownMenuItem
                  onClick={() => {
                    setSnap((prev) =>
                      prev
                        ? {
                            ...prev,
                            tasks: prev.tasks.map((t) => ({ ...t, status: 'pending', lastError: null })),
                            files: { ...prev.files, done: 0, error: 0, pending: prev.files.total, skipped: 0 },
                          }
                        : prev,
                    );
                    void datafulton.restartSession(sessionId);
                  }}
                >
                  <RotateCcw />Restart
                </DropdownMenuItem>
              )}
              <DropdownMenuItem
                disabled={session.status !== 'running' && session.status !== 'paused'}
                onClick={() => datafulton.cancelSession(sessionId)}
              >
                <CircleStop />Cancel
              </DropdownMenuItem>
              <DropdownMenuItem
                variant="destructive"
                disabled={session.status === 'running'}
                onClick={async () => {
                  if (confirm('Delete this extraction? (Exported files stay on disk.)')) {
                    await datafulton.deleteSession(sessionId);
                    onBack();
                  }
                }}
              >
                <Trash />Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </Toolbar>
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 px-5 py-5">
      <Card>
        <CardContent className="flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              <Badge variant={statusVariant(session.status)}>{session.status}</Badge>
              {running && <LoaderCircle className="size-4 animate-spin text-blue-500" />}
            </span>
            <span className="text-xs text-muted-foreground">{pct}%</span>
          </div>

          <SegmentedBar segs={taskSegs(snap)} />
          <div className="flex justify-between text-xs text-muted-foreground gap-3 flex-wrap">
            <span>
              {totalRecords.toLocaleString()} records · {tasks.filter((t) => t.status === 'done').length + files.done + files.skipped} done · {tasks.filter((t) => t.status === 'error').length + files.error} errors · {tasks.length + files.total} total
              {childTasks.length > 0 && ` · ${childDone}/${childTasks.length} per-job lookups`}
            </span>
            <button
              className="cursor-pointer truncate text-primary underline-offset-4 hover:underline"
              onClick={() => datafulton.openExportDir(sessionId)}
            >
              Open folder
            </button>
          </div>

          {(tasks.some((t) => t.lastError) || files.error > 0) && (
            <div className="flex flex-col gap-1 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-xs text-destructive">
              {tasks.filter((t) => t.lastError).map((t) => (
                <span key={t.id}>{t.label}: {t.lastError}</span>
              ))}
              {files.error > 0 && <span>{files.error} file(s) failed to download</span>}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader
          className="cursor-pointer select-none"
          onClick={() => setOpenCard(openCard === 'entities' ? 'log' : 'entities')}
        >
          <CardTitle className="flex items-center text-sm">
            Entities
            <ChevronDown className={`ml-auto size-4 text-muted-foreground transition-transform ${openCard === 'entities' ? 'rotate-180' : ''}`} />
          </CardTitle>
        </CardHeader>
        <AnimatePresence initial={false}>
        {openCard === 'entities' && (
        <motion.div
          key="entities"
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: 'auto', opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="overflow-hidden"
        >
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Data type</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entityTasks.map((t) => (
                <TableRow key={t.id}>
                  <TableCell>
                    {t.status === 'running' && <LoaderCircle className="mr-2 inline-block size-3.5 animate-spin text-blue-500" />}
                    {t.label}
                  </TableCell>
                  <TableCell>
                    {t.status === 'running' ? (
                      <span className="tabular-nums text-blue-400">{pct}%</span>
                    ) : (
                      <Badge variant={statusVariant(t.status)}>{t.status}</Badge>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
        </motion.div>
        )}
        </AnimatePresence>
      </Card>
      </div>
      </motion.div>
    )}
    </AnimatePresence>
  );
}
