# Multi-Project Sidebar — Design Spec

**Date:** 2026-03-25
**Status:** Draft
**Origin:** User feedback — richiesta di poter avere più progetti aperti contemporaneamente con tab in barra laterale.

---

## Summary

Aggiungere supporto multi-progetto a Reasonance con una sidebar verticale (project switcher) a sinistra del FileTree. Ogni progetto ha un contesto isolato (file, terminali, sessioni agent). Lo switch cambia l'intero workspace. I PTY restano vivi in background.

---

## Approach: A+ (Project Context Map con isolamento via namespace)

Nuovo store `projects` come `Map<projectId, Writable<ProjectContext>>` con un namespace layer che espone derived stores identici all'API attuale. I componenti consumer non sanno che esiste il concetto di multi-project — importano gli stessi store di oggi, che sotto il cofano puntano al progetto attivo.

### Why A+ over alternatives

- **vs Multi-Window (Tauri):** il feedback chiede tab in sidebar, non finestre separate. UX frammentata.
- **vs Webview per progetto:** isolamento totale ma RAM moltiplicata, IPC complesso, anti-pattern Tauri.
- **vs Context Map semplice (A):** A+ aggiunge un namespace layer che rende impossibile by design mostrare dati del progetto sbagliato. Costo: ~100-150 righe in più. Beneficio: safety a livello di tipo.

---

## 1. Data Model

### ProjectContext

```typescript
interface ProjectFileState {
  path: string;
  scrollPosition: { line: number; col: number };
  cursorPosition: { line: number; col: number };
  unsavedChanges: boolean;
}

interface ProjectContext {
  id: string;                          // UUID generato all'apertura
  rootPath: string;                    // path assoluto della cartella
  label: string;                       // nome cartella (derivato) o custom
  color: string;                       // colore assegnato automaticamente
  sortOrder: number;                   // per riordinamento manuale
  addedAt: number;                     // timestamp creazione
  pinned: boolean;                     // pinned in cima alla sidebar
  trustLevel: TrustLevel;
  openFiles: ProjectFileState[];
  activeFilePath: string | null;
  terminalInstances: TerminalInstance[];  // owned, non referenziati
  activeTerminalId: string | null;
  agentSessionIds: string[];
  fileTreeState: {
    collapsed: boolean;
    expandedDirs: string[];
  };
  gitState: {
    branch: string | null;
    hasChanges: boolean;
    remote: string | null;
  } | null;                            // null = non un repo git
}
```

### TrustLevel vs PermissionLevel — due concetti distinti

Il progetto ha due sistemi di autorizzazione separati:

- **WorkspaceTrust** (`trusted | read_only | blocked`): se la cartella e fidata per essere aperta. Definito in `workspace-trust.ts`. Si applica all'intero progetto.
- **PermissionLevel** (`yolo | ask | locked`): come l'agent LLM opera. Definito per-model nella config TOML. Non e per-progetto.

`ProjectContext` usa `WorkspaceTrust`, non `PermissionLevel`:

```typescript
import type { TrustLevel } from '$lib/stores/workspace-trust';

interface ProjectContext {
  // ...
  trustLevel: TrustLevel;  // 'trusted' | 'read_only' | 'blocked'
}
```

Il `PermissionLevel` resta per-model e non entra nel contesto progetto.

### OpenFile compatibility

L'esistente `OpenFile` in `files.ts`:
```typescript
interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
  isDeleted: boolean;
}
```

`ProjectFileState` estende il concetto per il multi-project:
```typescript
interface ProjectFileState {
  path: string;
  name: string;                        // derivato dal path (come ora)
  content: string;                     // contenuto in memoria
  isDirty: boolean;                    // ha modifiche non salvate
  isDeleted: boolean;                  // file eliminato dal filesystem
  scrollPosition: { line: number; col: number };
  cursorPosition: { line: number; col: number };
}
```

**Strategia memoria con N progetti:** i file aperti di tutti i progetti restano in memoria (campo `content`). Per progetti con molti file aperti, il consumo potrebbe crescere. Mitigazione futura: eviction LRU dei `content` per file non attivi da > 30 minuti, con re-read da disco al ripristino. Non implementato in v1 — monitorare via `ResourceMonitor`.

La migrazione da `OpenFile` a `ProjectFileState` aggiunge i campi `scrollPosition` e `cursorPosition` con valori default `{ line: 0, col: 0 }`.

### Store principale

```typescript
// Mappa piatta — singolo livello, no nested writables
const projects: Writable<Map<string, ProjectContext>>

// ID del progetto attivo
const activeProjectId: Writable<string | null>
```

**Nota:** si usa una mappa piatta (`Map<string, ProjectContext>`) invece di nested writables (`Map<string, Writable<ProjectContext>>`). Il pattern nested-writable crea problemi reali con le subscription Svelte (cleanup timing, race conditions su switch). Con la mappa piatta, i derived stores estraggono i dati del progetto attivo e l'equality check di Svelte previene re-render se i dati non sono cambiati. Modifiche a progetti background che non cambiano i valori estratti dal derived non causano re-render.

---

## 2. Namespace Layer

### Architettura

```
Componenti (FileTree, Editor, TerminalManager...)
  importano: projectRoot, openFiles, openFile(), etc.
        ↓
    NAMESPACE LAYER (derived stores + action wrappers)
        ↓
    PROJECT REGISTRY (Map<id, Writable<ProjectContext>>)
```

### Derived stores (lettura)

```typescript
// Derived diretto dalla mappa piatta — niente nested subscriptions
export const projectRoot: Readable<string> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => {
    if (!$id) return '';
    return $projects.get($id)?.rootPath ?? '';
  }
);

export const openFiles: Readable<ProjectFileState[]> = derived(
  [projects, activeProjectId],
  ([$projects, $id]) => {
    if (!$id) return [];
    return $projects.get($id)?.openFiles ?? [];
  }
);

// Stessa struttura per: activeFilePath, terminalInstances, activeTerminalId
```

Pattern semplice: ogni derived legge dalla mappa + activeProjectId. Svelte confronta il valore precedente — se il campo non e cambiato, nessun re-render. Modifiche a progetti background toccano la mappa ma non cambiano il valore estratto per il progetto attivo.

### Action wrappers (scrittura)

```typescript
export function openFile(path: string): void {
  const id = get(activeProjectId);
  if (!id) return;

  projects.update(map => {
    const ctx = map.get(id);
    if (!ctx) return map;  // progetto rimosso nel frattempo

    const alreadyOpen = ctx.openFiles.some(f => f.path === path);
    const updated = {
      ...ctx,
      openFiles: alreadyOpen
        ? ctx.openFiles
        : [...ctx.openFiles, {
            path,
            name: path.split('/').pop() ?? path,
            content: '',  // verrà letto dal filesystem
            isDirty: false,
            isDeleted: false,
            scrollPosition: { line: 0, col: 0 },
            cursorPosition: { line: 0, col: 0 },
          }],
      activeFilePath: path
    };
    return new Map(map).set(id, updated);
  });
}
```

### Garanzia di isolamento

Lo store `projects` (la mappa grezza) **non viene esportato**. I componenti consumer non possono accedere ai dati di un altro progetto.

Eccezione: la sidebar, che riceve un derived `projectSummaries` con dati summary di tutti i progetti.

### Project-level actions

```typescript
function addProject(rootPath: string): ProjectContext
function removeProject(id: string): void
function switchProject(id: string): void
function reorderProject(id: string, newSortOrder: number): void
```

### Store module structure

```
src/lib/stores/
  projects/
    types.ts          — ProjectContext, ProjectFileState, ProjectSummary
    registry.ts       — Map<id, ProjectContext>, add/remove
    namespace.ts      — derived stores consumer-facing, action wrappers
    persistence.ts    — session.json read/write/migrate
    sidebar.ts        — projectSummaries, projectStatuses
  index.ts            — re-export consumer API
```

---

## 3. Sidebar Component (ProjectSidebar.svelte)

### Layout

```
┌────┬──────────────┬──────────────┬───────────────┐
│ P  │              │              │               │
│ R  │  FILE TREE   │    EDITOR    │   TERMINAL    │
│ O  │              │              │               │
│ J  │              │              │               │
│    │              │              │               │
│ 48 │   ~200px     │    flex:1    │   ~300px      │
└────┴──────────────┴──────────────┴───────────────┘
```

Larghezza fissa 48px (da variabile tema `--sidebar-width`). Non resizable. Auto-hidden con un solo progetto.

### Struttura visiva

```
┌────┐
│ pin│  ← pinned section
│ [A]│  ← progetto attivo (bordo accent sinistro)
│ [B]│  ← progetto background (indicatori stato)
│────│  ← separator
│ [C]│  ← scrollable section
│ [D]│
│    │  ← scroll area
│ [+]│  ← aggiungi (browse / recenti)
└────┘
```

### Tab progetto

Ogni tab mostra:
- Iniziale del nome (o icona custom)
- Pallino colorato (`color` dal contesto)
- Bordo sinistro accent se attivo

### Indicatori di stato

| Indicatore | Visuale | Significato |
|------------|---------|-------------|
| Pallino pulsante | animato | Agent in esecuzione |
| Pallino statico | fisso | Terminali attivi, nessun agent |
| Cerchio vuoto | outline | Progetto idle |
| Pallino giallo | warning | File con unsaved changes |
| Badge numerico | sovrapposto | Errori o permission requests |

### Interazioni

| Azione | Comportamento |
|--------|--------------|
| Click tab | `switchProject(id)` |
| Drag tab | Riordina |
| Drag cartella da file manager | `addProject(path)` |
| Click "+" | Menu: browse / lista recenti |
| Right-click tab | Context menu: rinomina, colore, chiudi |
| Middle-click tab | Chiudi (con conferma se unsaved) |
| Hover tab | Tooltip: path completo + branch git |

### Aggiunta progetto

Tre modalità:
1. **Bottone "+"** in fondo alla sidebar → file picker
2. **Lista recenti** — il "+" mostra recenti + opzione browse
3. **Drag & drop** — cartella dal file manager sulla sidebar

### Prevenzione duplicati

`addProject()` normalizza il path (resolve symlink, trailing slash). Se già aperto: switch al progetto esistente + flash del tab.

### Collassamento

- Auto-collapse con un solo progetto (0px)
- Toggle manuale via toolbar button o `Ctrl+B`
- Stato persistito in `ui` store

### Accessibilita (WCAG AA)

- `role="tablist"` con `aria-orientation="vertical"`
- `role="tab"` + `aria-selected` su ogni progetto
- `aria-label` con nome progetto completo
- Navigazione tastiera: frecce su/giu, Enter per switch
- `Alt+1`...`Alt+9` per i primi 9 progetti (nota: `Ctrl+1..9` e gia usato per switch LLM provider in App.svelte)
- `Ctrl+Shift+E` per quick-switcher con ricerca fuzzy (nota: `Ctrl+Shift+P` e convenzionalmente "command palette" — evitare conflitto)
- Focus ring visibile
- Skip-link in App.svelte: aggiungere `#project-sidebar` alla lista skip-links esistente

### Stato vuoto (0 progetti)

Drop zone su tutta la finestra. Icona "+" grande + testo "Trascina una cartella o clicca per aprire".

### Chiusura progetto

1. Check unsaved files → dialog conferma
2. Check agent in esecuzione → dialog conferma
3. Kill PTY del progetto
4. Rimuovi file watcher
5. Salva stato finale per `recentProjects`
6. Rimuovi `ProjectContext`
7. Switch al progetto successivo (o stato vuoto se ultimo)

### Quick Switcher (ProjectQuickSwitcher.svelte)

Overlay attivato da `Ctrl+Shift+E`. Ricerca fuzzy tra progetti aperti. Utile con molti progetti.

### Overflow

Scroll verticale con scrollbar sottile. Sezione "pinned" fissa in alto, resto scrolla.

---

## 4. Switch Logic

### Cosa succede su switchProject(targetId)

```typescript
function switchProject(targetId: string): void {
  batch(() => {
    // 1. Salva stato UI del progetto uscente
    saveCurrentProjectUIState();
    //    - cursor/scroll positions di tutti i file
    //    - terminal scroll positions
    //    - tab terminale attiva
    //    - stato collapsed/expanded FileTree

    // 2. Cambia progetto attivo
    activeProjectId.set(targetId);

    // 3. Ripristina stato UI del progetto entrante
    restoreProjectUIState(targetId);
  });
  // I derived emettono UNA sola volta, stato finale coerente
}
```

Tempo percepito: quasi istantaneo — swap di stato Svelte, nessun spawn di processi.

### Batch update — implementazione concreta

Svelte non ha `batch()` nativo. Approccio scelto: **update atomico della mappa**.

Lo switch non modifica `activeProjectId` e lo stato UI separatamente. Invece:

1. `saveCurrentProjectUIState()` scrive cursor/scroll nell'entry del progetto uscente dentro la mappa `projects`
2. `activeProjectId.set(targetId)` — singola operazione atomica
3. I derived reagiscono: leggono i dati del nuovo progetto dalla mappa (che contiene gia lo stato UI salvato in precedenza)

Non c'e stato intermedio perche il cambio e un singolo `set()` su `activeProjectId`. Lo stato UI del progetto entrante e gia nella mappa (salvato allo switch precedente o al boot). L'unico side effect post-switch e il ripristino di scroll/cursor nell'editor DOM, che avviene via `tick().then(() => restoreEditorViewState())`.

Il flag `isSwitchingProject` non e piu necessario con questo approccio.

### Race condition su removeProject

Se `removeProject(id)` viene chiamato durante un'action asincrona per lo stesso progetto:
- Le action wrapper validano che il progetto esista ancora prima di operare
- Se il progetto rimosso era attivo, switch automatico al successivo

---

## 5. Persistenza

### session.json v2

```json
{
  "version": 2,
  "activeProjectId": "uuid-1",
  "sidebarCollapsed": false,
  "projects": {
    "uuid-1": {
      "rootPath": "/path/to/project-a",
      "label": "project-a",
      "color": "#4A9EFF",
      "sortOrder": 0,
      "pinned": true,
      "trustLevel": "ask",
      "openFiles": [
        {
          "path": "src/main.ts",
          "scrollPosition": { "line": 42, "col": 0 },
          "cursorPosition": { "line": 42, "col": 15 },
          "unsavedChanges": false
        }
      ],
      "activeFilePath": "src/main.ts",
      "terminalInstances": [],
      "activeTerminalId": null,
      "agentSessionIds": [],
      "fileTreeState": { "collapsed": false, "expandedDirs": ["src", "src/lib"] }
    }
  }
}
```

### Persistenza: migrazione da plugin-store a file custom

Attualmente la persistenza usa `@tauri-apps/plugin-store` (API key-value). Per il multi-project serve un formato piu strutturato con controllo sulla scrittura atomica.

**Decisione:** sostituire `plugin-store` con I/O file custom gestito da comandi Tauri Rust:
- `save_session(data: SessionV2)` — scrittura atomica (tmp + rename)
- `load_session() -> SessionV2` — lettura con fallback su backup
- `backup_session()` — copia corrente in backup

**Scrittura atomica (lato Rust):**
1. Serializza `SessionV2` in JSON
2. Scrivi su `session.tmp.json`
3. `rename()` su `session.json` (atomico su filesystem)
4. La versione precedente diventa `session.backup.json`

Debounced: 5s dopo l'ultimo cambiamento di stato (debounce gestito lato frontend, la chiamata Tauri e sincrona).

### Migrazione v1 → v2

Al primo avvio con la nuova versione:
1. Leggi session.json vecchio (senza `version` o `version === 1`)
2. Crea un `ProjectContext` con i dati flat esistenti
3. Wrappa nella struttura v2
4. Mantieni `session.v1.backup.json` come backup

Transizione trasparente, zero perdita di dati.

### Crash recovery

- Session.json scritto periodicamente (debounced 5s)
- PID file (`reasonance.pids`) traccia tutti i processi figlio attivi
- Al riavvio: legge session.json, ricrea contesti, cleanup processi orfani
- File con `unsavedChanges: true` → recovery dialog

---

## 6. Backend Tauri

### Multi-watcher

```rust
struct WatcherManager {
    active_watcher: Option<(String, RecommendedWatcher)>,  // real-time
    poll_watchers: HashMap<String, Instant>,                // background 30s
}
```

- Progetto attivo: watcher real-time ricorsivo
- Progetti background: polling ogni 30s
- Su switch: promuovi/degrada

### Inotify limit handling

1. Tenta watcher ricorsivo
2. Se inotify limit: fallback a watcher filtrato (ignora `node_modules`, `.git`, `target`, `dist`, `.next`, `__pycache__`, `.venv`)
3. Fallback estremo: polling 5s
4. Degradamento silenzioso con warning nel log

### PTY tagging

```rust
struct PtyProcess {
    id: String,
    project_id: String,
    project_root: PathBuf,
    child: Child,
}
```

`spawn_process()` riceve `project_id`, imposta `cwd` al `rootPath`.

### Path resolution esplicito

Ogni operazione file prende `projectId` esplicitamente. Il backend non si affida mai a uno stato "attivo" implicito per risolvere path. Il concetto di "progetto attivo" esiste solo per: watcher priority e UI.

### Trust level enforcement

Il backend enforza il workspace trust per-progetto su `spawn_process()`:
- `blocked`: rifiuta lo spawn
- `read_only`: consente solo operazioni di lettura
- `trusted`: procedi

Nota: il `PermissionLevel` (yolo/ask/locked) resta per-model nella config LLM e non e gestito qui. Vedi sezione 1 per la distinzione.

### Git state per progetto

Strategia a tre livelli:
- Progetto attivo: aggiorna su file watcher event (debounced 2s)
- Background con agent running: polling 15s
- Background idle: polling 60s

Usa `git status --porcelain` + cache basata su timestamp `.git/index`.

### Orphan PTY cleanup

Al boot:
1. Leggi `reasonance.pids`
2. Per ogni PID: check se esiste (`kill(pid, 0)`)
3. Se esiste: SIGTERM, attendi 3s, poi SIGKILL
4. Pulisci file temporanei

### Adapter interface

```typescript
interface Adapter {
  // Project lifecycle
  addProject(id: string, rootPath: string, trustLevel: TrustLevel): Promise<void>;
  removeProject(id: string): Promise<void>;
  setActiveProject(id: string): Promise<void>;

  // File ops — sempre con projectId esplicito
  readFile(projectId: string, path: string): Promise<string>;
  writeFile(projectId: string, path: string, content: string): Promise<void>;
  listDir(projectId: string, path: string): Promise<DirEntry[]>;

  // PTY
  spawnProcess(projectId: string, config: SpawnConfig): Promise<string>;
  killProcess(processId: string): Promise<void>;
  killProjectProcesses(projectId: string): Promise<void>;

  // Git
  getGitState(projectId: string): Promise<GitState | null>;

  // Resource monitoring
  getResourceUsage(): Promise<ResourceMonitor>;

  // Cleanup
  cleanupOrphanProcesses(): Promise<number>;
}
```

### Project status events

```rust
struct ProjectStatusEvent {
    project_id: String,
    event_type: ProjectEventType,  // FileChanged, ProcessExited, GitChanged
}
```

La sidebar ascolta questi eventi per aggiornare indicatori senza polling frontend.

### Resource monitoring

```rust
struct ResourceMonitor {
    pty_count: usize,
    watcher_count: usize,
    estimated_memory_mb: usize,
}
```

Nessun hard limit, ma feedback trasparente. Emesso ogni 30s.

---

## 7. Migrazione componenti esistenti

### Strategia incrementale a 3 fasi

**Fase 1 — Shim layer** (zero cambiamenti visibili)
- I vecchi store delegano a un ProjectContext singolo
- L'app funziona identicamente
- Verifica: tutti i test passano, comportamento invariato

**Fase 2 — Multi-project store** (backend pronto, UI single-project)
- Namespace layer attivo con un solo progetto nella mappa
- Tutti i componenti importano dal namespace
- `switchProject()` esiste ma non viene chiamato dalla UI
- Verifica: app funziona come prima con i nuovi store

**Fase 3 — Sidebar UI** (feature completa)
- Aggiunge ProjectSidebar, QuickSwitcher, AddMenu
- Switch attivo dalla UI
- Verifica: multi-project end-to-end

### Impatto per componente

| Componente | Cambiamento | Invasivita |
|------------|-------------|------------|
| App.svelte | Aggiunge ProjectSidebar, nuovo divider | Media |
| FileTree.svelte | Cambio import a namespace | Minima |
| Editor/EditorTabs | Cambio import a namespace | Minima |
| TerminalManager | Cambio import a namespace | Minima |
| ChatView | Filtra agent sessions per progetto attivo | Minima |
| StatusBar | Git state dal progetto attivo | Minima |
| Settings | Tab "Project" per config per-progetto | Media |
| Toolbar | Toggle sidebar button | Minima |
| +page.svelte | Startup multi-progetto, cleanup orfani | Media |

### Breaking change: Writable → Readable

`projectRoot`, `openFiles`, `activeFilePath` passano da `Writable` a `Readable`. Componenti che scrivono direttamente agli store devono migrare:

| Componente | Vecchia chiamata | Nuova chiamata |
|------------|-----------------|----------------|
| +page.svelte | `projectRoot.set(path)` | `restoreProjects(sessionData)` |
| Settings | `projectRoot.set(path)` | `switchProject(id)` o `addProject(path)` |
| files.ts `switchProject()` | modifica diretta store | eliminata, sostituita da `namespace.switchProject()` |

### HIVE workflows

I workflow HIVE dovranno essere associati a un progetto:

```typescript
interface HiveWorkflow {
  // ...
  projectId?: string;  // opzionale per backward compat
}
```

Warning se l'utente apre un workflow creato per un altro progetto.

### Componenti mancanti nella matrice

Oltre ai componenti gia elencati, servono modifiche a:

| Componente | Cambiamento | Invasivita |
|------------|-------------|------------|
| SearchPalette.svelte | Scoping ricerca per progetto attivo (usa projectRoot dal namespace) | Minima |
| FindInFiles.svelte | Scoping ricerca per progetto attivo | Minima |
| ChatView.svelte | Filtra `agentSessions` per `activeProject.agentSessionIds` — mostra solo sessioni del progetto attivo | Minima |

### Componenti nuovi

| Componente | Responsabilita |
|------------|----------------|
| ProjectSidebar.svelte | Colonna tab, drag & drop, context menu, indicatori |
| ProjectQuickSwitcher.svelte | Overlay Ctrl+Shift+E, ricerca fuzzy |
| ProjectAddMenu.svelte | Menu "+": browse, recenti |
| ProjectDisconnectedDialog.svelte | "Cartella non trovata" — al boot E a runtime (es. USB rimosso, mount caduto) |

### Matrice di test

| Test | Fase 1 | Fase 2 | Fase 3 |
|------|--------|--------|--------|
| Apri app, FileTree mostra file | pass | pass | pass |
| Apri file, editor lo mostra | pass | pass | pass |
| Crea terminale, funziona | pass | pass | pass |
| Chiudi e riapri app, stato ripristinato | pass | pass | pass |
| Session.json migrato correttamente | — | pass | pass |
| Switch progetto, UI aggiorna | — | — | pass |
| PTY background continua | — | — | pass |
| Drag cartella su sidebar | — | — | pass |
| Chiudi progetto, cleanup PTY | — | — | pass |
| Crash recovery con N progetti | — | — | pass |
| Progetto con cartella mancante | — | — | pass |

---

## 8. Theme System Updates

### Principio: la sidebar e un componente come tutti gli altri

Nessun trattamento speciale. Le variabili sidebar sono required nel tema, come qualsiasi altro componente.

### Nuove variabili CSS

**Sezione `colors`:**
```json
{
  "--sidebar-bg": "value",
  "--sidebar-bg-hover": "value",
  "--sidebar-border": "value",
  "--sidebar-tab-active-accent": "value",
  "--sidebar-tab-active-bg": "value",
  "--sidebar-tab-text": "value",
  "--sidebar-tab-text-active": "value"
}
```

**Sezione `states`:**
```json
{
  "--sidebar-indicator-running": "value",
  "--sidebar-indicator-idle": "value",
  "--sidebar-indicator-unsaved": "value",
  "--sidebar-indicator-error": "value",
  "--sidebar-indicator-pulse": "value"
}
```

**Sezione `ui-states`:**
```json
{
  "--sidebar-dropzone-bg": "value",
  "--sidebar-dropzone-border": "value",
  "--sidebar-separator": "value",
  "--sidebar-badge-bg": "value",
  "--sidebar-badge-text": "value"
}
```

**Sezione `layout`:**
```json
{
  "--sidebar-width": "48px"
}
```

**Sezione `transitions`:**
```json
{
  "--sidebar-transition-speed": "150ms"
}
```

### File da aggiornare

| File | Cosa |
|------|------|
| `theme-schema.json` | Variabili sidebar required |
| `reasonance-dark.json` | Valori dark |
| `reasonance-light.json` | Valori light |
| `fallback-theme.ts` | Valori hardcoded |
| `enhanced-readability.json` | Override spacing/font-size badge |
| `_high-contrast.json` | Override bordi/contrasto |
| `_reduced-motion.json` | No pulse, no transition |
| `theme-validator.ts` | Validazione v2 con sidebar required |
| `theme-engine.ts` | Migrazione temi utente v1 → v2 |
| `ThemeStartDialog.svelte` | Template nuovo tema include sidebar |

### Schema version bump

`schemaVersion` da `1` a `2`. Temi utente v1 migrati automaticamente:
1. Al caricamento di un tema v1: inietta variabili sidebar con valori default (dark/light in base a colorScheme)
2. Aggiorna schemaVersion a 2
3. Salva su disco
4. Notifica utente

---

## 9. Nessun limite progetti

Nessun hard limit. Resource monitoring passivo con feedback trasparente via `ResourceMonitor`. La sidebar scrolla per overflow, con sezione pinned fissa.

---

## 10. Dettagli aggiuntivi

### recentProjects store

L'esistente `recentProjects: Writable<string[]>` in `files.ts` viene assorbito nel sistema multi-project:
- I progetti aperti sono nella mappa `projects`
- I progetti chiusi vengono aggiunti a `recentProjects` (con path + label + color per mostrare nel menu "+")
- `recentProjects` viene migrato nella session v2 come campo top-level (non dentro un progetto)
- L'esistente `addRecentProject()` e `switchProject()` in files.ts vengono eliminati e sostituiti dalle action del namespace

### fs-change event routing

L'evento `fs-change` attuale porta solo path e tipo. Con multi-project:
- Il backend include `project_id` nell'evento: `{ project_id: String, path: String, event_type: String }`
- Il frontend riceve l'evento e lo instrada al `ProjectContext` corretto
- Se il progetto e attivo: FileTree aggiorna in real-time
- Se il progetto e background: aggiorna solo il `gitState` e gli indicatori sidebar

### sortOrder — algoritmo di riordinamento

Quando un progetto viene trascinato tra due altri:
1. `newSortOrder = (sortOrderBefore + sortOrderAfter) / 2` (inserimento frazionale)
2. Se la precisione floating point diventa insufficiente (dopo ~50 riordinamenti nello stesso gap), ricalcola tutti i sortOrder come interi equidistanti (0, 1000, 2000, ...)
3. Questo evita di ricalcolare tutti gli ordini ad ogni drag

### Progetto disconnesso a runtime

Oltre al boot, una cartella puo sparire durante l'uso (USB rimosso, mount di rete caduto):
- Il file watcher emette un errore → il backend emette `ProjectStatusEvent { event_type: Disconnected }`
- Il tab nella sidebar mostra stato "disconnesso" (icona grigia, bordo tratteggiato)
- Se il progetto disconnesso e attivo, mostra un banner non-blocking nell'editor: "La cartella [path] non e raggiungibile. Ricollegati o chiudi il progetto."
- I file gia in memoria restano accessibili (read-only)
- Quando la cartella torna disponibile, il watcher si riconnette automaticamente e il banner scompare

### Rust commands da modificare

Comandi Tauri esistenti che assumono un singolo `ProjectRootState` e devono essere migrati a `ProjectsState` (mappa):

| Comando | Cambiamento |
|---------|-------------|
| `set_project_root` | Sostituito da `add_project` + `set_active_project` |
| `start_watching` / `stop_watching` | Sostituiti da `WatcherManager` multi-progetto |
| `read_file` / `write_file` / `list_dir` | Aggiungere parametro `project_id`, risolvere path da mappa |
| `spawn_pty` | Aggiungere parametro `project_id`, impostare cwd |
| `get_git_status` | Aggiungere parametro `project_id` |

Lo stato Rust passa da:
```rust
struct ProjectRootState(Mutex<Option<PathBuf>>);
```
a:
```rust
struct ProjectsState(Mutex<HashMap<String, ProjectState>>);
struct ActiveProjectState(Mutex<Option<String>>);
```

### Drag-and-drop Tauri config

Per supportare il drag-and-drop di cartelle esterne, verificare e aggiornare `tauri.conf.json`:
- `windows[0].dragDropEnabled` deve essere `true` (default in Tauri v2)
- Il frontend ascolta l'evento `tauri://drag-drop` e filtra per directory (ignora file singoli)

### Agent session tagging

`AgentSessionState` in `agent-session.ts` non ha un campo `projectId`. Quando ChatView crea una sessione via `upsertSession()`, deve passare il `projectId` attivo.

Modifica:
```typescript
// agent-session.ts
interface AgentSessionState {
  // campi esistenti...
  projectId: string;  // nuovo
}

// ChatView.svelte — dove chiama upsertSession()
upsertSession({
  // campi esistenti...
  projectId: get(activeProjectId),  // inietta il progetto attivo
});
```

Il namespace layer espone un derived `agentSessionIds` che filtra le sessioni globali per il progetto attivo. ChatView usa questo derived per mostrare solo le sessioni pertinenti.

### Terminal buffer — limitazione xterm.js

Il terminale usa **xterm.js** con `SerializeAddon`. Il serialize produce solo testo piano — **non** lo stato completo del buffer (colori, cursore, scroll position). Conseguenze:

- **Progetto attivo:** il terminale vive, buffer integro, nessun problema
- **Progetto background:** i PTY restano vivi, il buffer xterm.js viene distrutto quando il componente Terminal.svelte si smonta
- **Al ritorno sul progetto:** il terminale si rimonta e riceve l'output futuro dal PTY, ma il buffer storico e perso

**Mitigazione:**
1. **Non smontare** i terminali dei progetti background — nasconderli con `display: none` o `visibility: hidden`. Costoso in memoria DOM ma preserva il buffer completo.
2. **Alternativa:** prima di smontare, serializzare con `SerializeAddon.serialize()` (testo piano) e re-scrivere nel terminale al rimontaggio via `term.write(serialized)`. Perde i colori ANSI ma preserva il contenuto testuale.
3. **Raccomandazione per v1:** opzione 1 (non smontare). L'overhead DOM e accettabile con un numero ragionevole di terminali. Se diventa un problema (10+ terminali per progetto), implementare opzione 2 come fallback.

### Window title dinamico

Il titolo della finestra Tauri e statico ("Reasonance"). Con multi-project, deve riflettere il progetto attivo:

```typescript
// Reagisce ai cambiamenti del progetto attivo
$: if ($projectRoot) {
  const label = $projectRoot.split('/').pop() ?? 'Reasonance';
  getCurrentWindow().setTitle(`${label} — Reasonance`);
}
```

Implementare in `App.svelte` come effetto reattivo sul derived `projectRoot` dal namespace.

### MenuBar aggiornamenti

`MenuBar.svelte` ha gia "Open Folder" (dispatcha `reasonance:openFolder`) e un submenu "Recent" (placeholder vuoto). Modifiche:

| Voce menu | Ora | Dopo |
|-----------|-----|------|
| Open Folder | Apre dialog, switcha progetto unico | Apre dialog, aggiunge progetto alla sidebar |
| Recent | Placeholder `(none)` | Popola con `recentProjects`, click = `addProject(path)` |
| **Close Project** | Non esiste | **Nuovo** — chiude il progetto attivo |

L'handler `reasonance:openFolder` in `+page.svelte` va aggiornato per chiamare `addProject()` invece di `switchProject()` del vecchio sistema.

### CLI arguments

Il plugin `single-instance` in `lib.rs` riceve gli argomenti CLI ma li ignora (`_args`). Con multi-project:

```rust
.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
    // Se passato un path, emetti evento al frontend
    if args.len() > 1 {  // args[0] e il nome dell'eseguibile
        if let Some(path) = args.get(1) {
            let _ = app.emit("cli-open-project", path);
        }
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_focus();
    }
}))
```

Frontend: ascolta `cli-open-project` e chiama `addProject(path)`. Funziona sia al primo avvio che quando l'app e gia aperta (second instance).

### computedLabels scoping

`computedLabels` in `terminals.ts` genera labels globali ("Claude 1", "Claude 2"). Con multi-project, le labels devono essere scoped per progetto — se progetto A ha "Claude 1" e progetto B ha un Claude, quello di B deve essere "Claude 1" (non "Claude 3").

Il namespace layer espone `terminalInstances` gia filtrate per progetto attivo. Il `computedLabels` derived va spostato nel namespace e derivato da `terminalInstances` (filtrate) invece che globali.

### PTY event routing — gia gestito

Gli eventi PTY usano il pattern `pty-data-{instanceId}` con ID univoci per istanza. Ogni componente Terminal.svelte ascolta solo il suo `ptyId`. Questo funziona gia con multi-project senza modifiche — l'ID e globalmente univoco, non dipende dal progetto.

---

## Open Questions

Nessuna — tutti i requisiti e i dettagli implementativi sono stati definiti.
