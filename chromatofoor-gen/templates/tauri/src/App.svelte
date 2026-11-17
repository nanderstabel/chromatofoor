<script lang="ts">
  import { onMount } from "svelte";

  const isTauri =
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

  async function tauriInvoke<T = any>(
    cmd: string,
    args?: Record<string, unknown>,
  ): Promise<T> {
    const internals = (window as any).__TAURI_INTERNALS__;
    if (internals && typeof internals.invoke === "function") {
      return internals.invoke(cmd, args);
    }
    const modName = "@tauri-apps/api/core";
    const mod = await import(/* @vite-ignore */ modName);
    return mod.invoke(cmd, args);
  }

  async function parseHttpError(res: Response): Promise<string> {
    try {
      const body = await res.json();
      if (body && typeof body === "object") {
        const msg = body.detail || body.title || body.error || res.statusText;
        return body.code ? `[${body.code}] ${msg}` : String(msg);
      }
    } catch {
      // ignore non-JSON error body
    }
    return `${res.status} ${res.statusText}`;
  }

  interface CommandDesc {
    kind: string;
    bounded_context: string;
    target: string;
    target_kind: string;
    name: string;
    topic: string;
    description: string;
    allowed_actors: string[];
    schema: any;
  }

  interface QueryDesc {
    kind: string;
    bounded_context: string;
    read_model: string;
    name: string;
    topic: string;
    description: string;
    allowed_actors: string[];
    parameters_schema: any;
    output_schema: any;
  }

  interface ReadModelDesc {
    kind: string;
    bounded_context: string;
    name: string;
    topic: string;
    description: string;
    events: string[];
  }

  interface ConsumerDesc {
    kind: string;
    name: string;
    topic: string;
    description: string;
  }

  interface DomainCatalog {
    domain: string;
    topic: string;
    description: string;
    actors: string[];
    commands: CommandDesc[];
    queries: QueryDesc[];
    read_models: ReadModelDesc[];
    consumers: ConsumerDesc[];
  }

  interface StoredEvent {
    id: string;
    subject: string;
    instance_id: string;
    type: string;
    event_name: string;
    source: string;
    time: string;
    data: Record<string, any>;
  }

  interface KnownInstance {
    id: string;
    aggregate: string;
    bounded_context: string;
    label: string;
    last_event: string;
    time: string;
  }

  type WorkspaceTab = "canvas" | "workbench";
  type WorkbenchMode = "commands" | "queries";

  interface InspectorSelection {
    kind: "command" | "aggregate" | "read-model" | "query" | "consumer";
    bounded_context: string;
    name: string;
    target?: string;
    description: string;
    topic: string;
    allowed_actors?: string[];
    schema?: any;
    output_schema?: any;
    events?: string[];
    commandRef?: CommandDesc;
    queryRef?: QueryDesc;
  }

  // Runtime state
  let connected = $state(false);
  let loading = $state(true);
  let errorBanner = $state<string | null>(null);
  let catalog = $state<DomainCatalog>({
    domain: "chromatofoor-app",
    topic: "/chromatofoor-app",
    description: "",
    actors: ["user"],
    commands: [],
    queries: [],
    read_models: [],
    consumers: [],
  });
  let events = $state<StoredEvent[]>([]);

  // Navigation & Filters
  let activeTab = $state<WorkspaceTab>("workbench");
  let activeActor = $state("user");
  let selectedContext = $state<string>("all");
  let workbenchMode = $state<WorkbenchMode>("commands");
  let eventFilter = $state("");

  // Selected Command / Query in Workbench
  let selectedCommand = $state<CommandDesc | null>(null);
  let selectedQuery = $state<QueryDesc | null>(null);
  let commandFormValues = $state<Record<string, any>>({});
  let queryFormValues = $state<Record<string, any>>({});
  let rawJsonMode = $state(false);
  let rawCommandJson = $state("{}");
  let rawQueryJson = $state("{}");

  // Execution Feedback
  let isSubmitting = $state(false);
  let commandResult = $state<{
    ok: boolean;
    command: string;
    aggregate: string;
    bounded_context: string;
    emittedEvents: StoredEvent[];
    error?: string;
  } | null>(null);
  let queryResult = $state<{
    ok: boolean;
    query: string;
    read_model: string;
    bounded_context: string;
    params: Record<string, any>;
    data?: any;
    error?: string;
  } | null>(null);

  // Canvas Inspector Selection
  let inspectorItem = $state<InspectorSelection | null>(null);

  // Derived: Bounded Contexts list
  let boundedContexts = $derived.by(() => {
    const set = new Set<string>();
    for (const c of catalog.commands) set.add(c.bounded_context);
    for (const q of catalog.queries) set.add(q.bounded_context);
    for (const rm of catalog.read_models) set.add(rm.bounded_context);
    return Array.from(set).sort();
  });

  // Derived: Filtered Commands & Queries
  let filteredCommands = $derived.by(() => {
    return catalog.commands.filter((c) => {
      const ctxMatch = selectedContext === "all" || c.bounded_context === selectedContext;
      const actorMatch =
        !activeActor ||
        c.allowed_actors.length === 0 ||
        c.allowed_actors.includes(activeActor);
      return ctxMatch && actorMatch;
    });
  });

  let filteredQueries = $derived.by(() => {
    return catalog.queries.filter((q) => {
      const ctxMatch = selectedContext === "all" || q.bounded_context === selectedContext;
      const actorMatch =
        !activeActor ||
        q.allowed_actors.length === 0 ||
        q.allowed_actors.includes(activeActor);
      return ctxMatch && actorMatch;
    });
  });

  // Derived: Reverse-chronological filtered events
  let displayedEvents = $derived.by(() => {
    const q = eventFilter.trim().toLowerCase();
    const list = [...events].reverse();
    if (!q) return list;
    return list.filter(
      (ev) =>
        ev.event_name.toLowerCase().includes(q) ||
        ev.subject.toLowerCase().includes(q) ||
        ev.instance_id.toLowerCase().includes(q) ||
        JSON.stringify(ev.data).toLowerCase().includes(q)
    );
  });

  // Derived: Known Aggregate Instances extracted from the real Event Stream
  let knownInstances = $derived.by(() => {
    const map = new Map<string, KnownInstance>();
    for (const ev of events) {
      // Subject format: /<domain>/<bounded_context>/<aggregate>/<instance_id>
      const parts = ev.subject.split("/").filter(Boolean);
      if (parts.length >= 4) {
        const bc = parts[1];
        const agg = parts[2];
        const id = parts[parts.length - 1];
        const existing = map.get(id);
        let label = existing?.label || `${agg} (${id.slice(0, 8)})`;
        if (ev.data && typeof ev.data === "object") {
          if (typeof ev.data.name === "string" && ev.data.name.trim()) {
            label = `${ev.data.name} (${id.slice(0, 8)})`;
          } else if (typeof ev.data["user-id"] === "string") {
            label = `${ev.data["user-id"]} (${id.slice(0, 8)})`;
          }
        }
        map.set(id, {
          id,
          aggregate: agg,
          bounded_context: bc,
          label,
          last_event: ev.event_name,
          time: ev.time,
        });
      }
    }
    return Array.from(map.values()).reverse();
  });

  // Helper: Resolve JSON Schema node
  function resolveSchemaNode(node: any, root: any): any {
    if (!node || typeof node !== "object") return {};
    if (typeof node.$ref === "string") {
      const defName = node.$ref
        .replace(/^#\/\$defs\//, "")
        .replace(/^#\/definitions\//, "");
      const resolved = root?.$defs?.[defName] ?? root?.definitions?.[defName];
      if (resolved) return resolveSchemaNode(resolved, root);
    }
    if (Array.isArray(node.allOf) && node.allOf.length === 1) {
      return resolveSchemaNode(node.allOf[0], root);
    }
    const branches = node.anyOf ?? node.oneOf;
    if (Array.isArray(branches)) {
      const nonNull = branches.find((b: any) => b?.type !== "null");
      if (nonNull) return resolveSchemaNode(nonNull, root);
    }
    return node;
  }

  interface SchemaField {
    name: string;
    type: string;
    format?: string;
    required: boolean;
    enumValues?: string[];
    nullable: boolean;
  }

  function extractSchemaFields(schema: any): SchemaField[] {
    if (!schema || typeof schema !== "object") return [];
    const root = resolveSchemaNode(schema, schema);
    const props = root?.properties;
    if (!props || typeof props !== "object") return [];
    const requiredSet = new Set<string>(Array.isArray(root.required) ? root.required : []);

    return Object.entries(props).map(([name, rawProp]: [string, any]) => {
      const resolved = resolveSchemaNode(rawProp, schema);
      let type = "string";
      let nullable = false;
      if (typeof resolved.type === "string") {
        type = resolved.type;
      } else if (Array.isArray(resolved.type)) {
        nullable = resolved.type.includes("null");
        type = resolved.type.find((t: string) => t !== "null") || "string";
      }
      if (Array.isArray(rawProp?.anyOf) || Array.isArray(rawProp?.oneOf)) {
        const branches = rawProp.anyOf ?? rawProp.oneOf;
        if (branches.some((b: any) => b?.type === "null")) {
          nullable = true;
        }
      }
      return {
        name,
        type,
        format: resolved.format,
        required: requiredSet.has(name) && !nullable,
        enumValues: Array.isArray(resolved.enum) ? resolved.enum : undefined,
        nullable,
      };
    });
  }

  function generateUuid(): string {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
    return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      const v = c === "x" ? r : (r & 0x3) | 0x8;
      return v.toString(16);
    });
  }

  function defaultFieldValue(field: SchemaField, targetAggregate?: string): any {
    if (field.enumValues && field.enumValues.length > 0) {
      return field.enumValues[0];
    }
    if (field.type === "boolean") return false;
    if (field.type === "integer" || field.type === "number") return 0;
    if (field.type === "array") return "[]";
    if (field.type === "object") return "{}";

    // Auto-fill from known instances if appropriate
    const isUuidField =
      field.format === "uuid" ||
      field.name === "id" ||
      field.name.endsWith("-id") ||
      field.name.endsWith("_id");

    if (isUuidField) {
      if (field.name === "id" && targetAggregate) {
        const match = knownInstances.find((i) => i.aggregate === targetAggregate);
        if (match) return match.id;
      }
      if (field.name === "workspace-id" || field.name === "workspace_id") {
        const match = knownInstances.find((i) => i.aggregate === "workspace");
        if (match) return match.id;
      }
      if (field.name === "project-id" || field.name === "project_id") {
        const match = knownInstances.find((i) => i.aggregate === "project");
        if (match) return match.id;
      }
      if (field.name === "job-id" || field.name === "job_id") {
        return generateUuid();
      }
      return generateUuid();
    }

    // Helpful defaults for sample fields
    if (field.name === "name") {
      if (targetAggregate === "workspace") return "Core Domain Workspace";
      if (targetAggregate === "project") return "Order Management System";
      return "Sample Name";
    }
    if (field.name === "owner-id") return "architect@chromatofoor.io";
    if (field.name === "email" || field.name === "member-id") return "engineer@chromatofoor.io";
    if (field.name === "user-id") return "architect@chromatofoor.io";
    if (field.name === "content") return "Verify aggregate consistency boundary";
    if (field.name === "color") return "amber";
    if (field.name === "description") return "Event-sourced domain model";
    if (field.name === "yaml-content" || field.name === "suggestions-yaml") {
      return "apiVersion: schema.esdm.io/core/v1\nkind: domain\nname: order-management";
    }
    if (field.name === "output-zip-url") return "/artifacts/workspace.zip";
    if (field.name === "error-message") return "Schema validation error";
    if (field.name === "target-environment") return "production-eu-west";
    if (field.name === "user-prompt" || field.name === "prompt") {
      return "Add a discount voucher command to the Order aggregate";
    }
    if (field.name === "target-file") return "src/domain/order/command/place_order.rs";
    if (field.name === "context") return "Order aggregate command handler";
    if (field.name === "code-snippet") return "Ok(vec![OrderEvent::OrderPlaced(ev)])";

    return "";
  }

  function selectCommandItem(cmd: CommandDesc) {
    selectedCommand = cmd;
    workbenchMode = "commands";
    const fields = extractSchemaFields(cmd.schema);
    const nextVals: Record<string, any> = {};
    for (const f of fields) {
      nextVals[f.name] = defaultFieldValue(f, cmd.target);
    }
    commandFormValues = nextVals;
    rawCommandJson = JSON.stringify(buildPayloadFromForm(fields, nextVals), null, 2);
  }

  function selectQueryItem(q: QueryDesc, prefillId?: string) {
    selectedQuery = q;
    workbenchMode = "queries";
    const fields = extractSchemaFields(q.parameters_schema);
    const nextVals: Record<string, any> = {};
    for (const f of fields) {
      if (prefillId && (f.name === "id" || f.name.endsWith("id"))) {
        nextVals[f.name] = prefillId;
      } else {
        // Match read_model to aggregate if possible
        const relatedAgg = catalog.commands.find(
          (c) => c.bounded_context === q.bounded_context
        )?.target;
        const match = knownInstances.find(
          (i) => i.bounded_context === q.bounded_context || i.aggregate === relatedAgg
        );
        nextVals[f.name] = match ? match.id : defaultFieldValue(f, relatedAgg);
      }
    }
    queryFormValues = nextVals;
    rawQueryJson = JSON.stringify(buildPayloadFromForm(fields, nextVals), null, 2);
  }

  function buildPayloadFromForm(
    fields: SchemaField[],
    values: Record<string, any>
  ): Record<string, any> {
    const out: Record<string, any> = {};
    for (const f of fields) {
      const raw = values[f.name];
      if ((raw === "" || raw === undefined || raw === null) && f.nullable) {
        out[f.name] = null;
        continue;
      }
      if (f.type === "integer") {
        out[f.name] = Number.parseInt(String(raw), 10) || 0;
      } else if (f.type === "number") {
        out[f.name] = Number.parseFloat(String(raw)) || 0;
      } else if (f.type === "boolean") {
        out[f.name] = Boolean(raw);
      } else if (f.type === "array" || f.type === "object") {
        if (typeof raw === "string") {
          try {
            out[f.name] = JSON.parse(raw);
          } catch {
            out[f.name] = raw;
          }
        } else {
          out[f.name] = raw;
        }
      } else {
        out[f.name] = raw ?? "";
      }
    }
    return out;
  }

  function syncCommandRawJson() {
    if (!selectedCommand) return;
    const fields = extractSchemaFields(selectedCommand.schema);
    rawCommandJson = JSON.stringify(buildPayloadFromForm(fields, commandFormValues), null, 2);
  }

  function syncQueryRawJson() {
    if (!selectedQuery) return;
    const fields = extractSchemaFields(selectedQuery.parameters_schema);
    rawQueryJson = JSON.stringify(buildPayloadFromForm(fields, queryFormValues), null, 2);
  }

  async function refreshEvents() {
    try {
      const res: any = isTauri
        ? await tauriInvoke("get_events")
        : await fetch("/v0/events/history").then(async (r) => {
            if (!r.ok) throw new Error(await parseHttpError(r));
            return r.json();
          });
      if (Array.isArray(res)) {
        events = res;
      }
    } catch (err: any) {
      console.warn("Failed to load events:", err);
    }
  }

  async function loadCatalogAndEvents() {
    loading = true;
    errorBanner = null;
    try {
      const res: any = isTauri
        ? await tauriInvoke("get_schema")
        : await fetch("/v0/catalog").then(async (r) => {
            if (!r.ok) throw new Error(await parseHttpError(r));
            return r.json();
          });
      if (res && typeof res === "object") {
        catalog = {
          domain: res.domain || "chromatofoor-app",
          topic: res.topic || "/chromatofoor-app",
          description: res.description || "",
          actors: Array.isArray(res.actors) && res.actors.length > 0 ? res.actors : ["user"],
          commands: Array.isArray(res.commands) ? res.commands : [],
          queries: Array.isArray(res.queries) ? res.queries : [],
          read_models: Array.isArray(res.read_models) ? res.read_models : [],
          consumers: Array.isArray(res.consumers) ? res.consumers : [],
        };
        if (catalog.actors.length > 0 && !catalog.actors.includes(activeActor)) {
          activeActor = catalog.actors[0];
        }
        connected = true;
      }
      await refreshEvents();

      if (!selectedQuery && catalog.queries.length > 0) {
        selectQueryItem(catalog.queries[0]);
      }
      if (!selectedCommand && catalog.commands.length > 0) {
        const defaultCmd =
          catalog.commands.find((c) => c.name === "create-workspace") || catalog.commands[0];
        selectCommandItem(defaultCmd);
      }
    } catch (err: any) {
      connected = false;
      errorBanner = String(err);
    } finally {
      loading = false;
    }
  }

  async function handleDispatchCommand() {
    if (!selectedCommand) return;
    isSubmitting = true;
    commandResult = null;
    try {
      const fields = extractSchemaFields(selectedCommand.schema);
      const payload = rawJsonMode
        ? JSON.parse(rawCommandJson)
        : buildPayloadFromForm(fields, commandFormValues);

      let emitted: StoredEvent[] = [];
      if (isTauri) {
        const res: any = await tauriInvoke("dispatch_command", {
          ctx: selectedCommand.bounded_context,
          agg: selectedCommand.target,
          cmdName: selectedCommand.name,
          payload,
          metadata: {
            actor_type: activeActor,
          },
        });
        emitted = Array.isArray(res?.events) ? res.events : [];
        await refreshEvents();
      } else {
        const url = `/v0/${encodeURIComponent(selectedCommand.bounded_context)}/${encodeURIComponent(selectedCommand.target)}/${encodeURIComponent(selectedCommand.name)}`;
        const httpRes = await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Actor": activeActor,
          },
          body: JSON.stringify(payload),
        });
        if (!httpRes.ok) {
          throw new Error(await parseHttpError(httpRes));
        }
        const count = await httpRes.json();
        await refreshEvents();
        if (typeof count === "number" && count > 0) {
          emitted = events.slice(-count);
        }
      }

      commandResult = {
        ok: true,
        command: selectedCommand.name,
        aggregate: selectedCommand.target,
        bounded_context: selectedCommand.bounded_context,
        emittedEvents: emitted,
      };
    } catch (err: any) {
      commandResult = {
        ok: false,
        command: selectedCommand.name,
        aggregate: selectedCommand.target,
        bounded_context: selectedCommand.bounded_context,
        emittedEvents: [],
        error: String(err),
      };
    } finally {
      isSubmitting = false;
    }
  }

  async function handleExecuteQuery() {
    if (!selectedQuery) return;
    isSubmitting = true;
    queryResult = null;
    try {
      const fields = extractSchemaFields(selectedQuery.parameters_schema);
      const params = rawJsonMode
        ? JSON.parse(rawQueryJson)
        : buildPayloadFromForm(fields, queryFormValues);

      let res: any;
      if (isTauri) {
        res = await tauriInvoke("query_read_model", {
          ctx: selectedQuery.bounded_context,
          queryName: selectedQuery.name,
          params,
          metadata: {
            actor_type: activeActor,
          },
        });
      } else {
        const qs = new URLSearchParams();
        if (params && typeof params === "object") {
          for (const [k, v] of Object.entries(params)) {
            if (v !== undefined && v !== null) {
              qs.set(k, typeof v === "string" ? v : JSON.stringify(v));
            }
          }
        }
        const queryString = qs.toString();
        const url = `/v0/${encodeURIComponent(selectedQuery.bounded_context)}/${encodeURIComponent(selectedQuery.name)}${queryString ? `?${queryString}` : ""}`;
        const httpRes = await fetch(url, {
          headers: {
            "X-Actor": activeActor,
          },
        });
        if (!httpRes.ok) {
          throw new Error(await parseHttpError(httpRes));
        }
        res = await httpRes.json();
      }

      queryResult = {
        ok: true,
        query: selectedQuery.name,
        read_model: selectedQuery.read_model,
        bounded_context: selectedQuery.bounded_context,
        params,
        data: res,
      };
    } catch (err: any) {
      queryResult = {
        ok: false,
        query: selectedQuery.name,
        read_model: selectedQuery.read_model,
        bounded_context: selectedQuery.bounded_context,
        params: queryFormValues,
        error: String(err),
      };
    } finally {
      isSubmitting = false;
    }
  }

  function jumpToQueryWithInstance(boundedContext: string, instanceId: string) {
    const q = catalog.queries.find((item) => item.bounded_context === boundedContext);
    if (!q) return;
    activeTab = "workbench";
    selectQueryItem(q, instanceId);
    handleExecuteQuery();
  }

  function applyInstanceIdToActiveForm(id: string, aggregateName?: string) {
    if (workbenchMode === "commands" && selectedCommand) {
      const fields = extractSchemaFields(selectedCommand.schema);
      const next = { ...commandFormValues };
      if (
        aggregateName === "workspace" &&
        fields.some((f) => f.name === "workspace-id") &&
        selectedCommand.target !== "workspace"
      ) {
        next["workspace-id"] = id;
      } else if (
        aggregateName === "project" &&
        fields.some((f) => f.name === "project-id") &&
        selectedCommand.target !== "project"
      ) {
        next["project-id"] = id;
      } else if (fields.some((f) => f.name === "id")) {
        next["id"] = id;
      } else {
        const uuidField = fields.find(
          (f) => f.format === "uuid" || f.name.endsWith("-id") || f.name.endsWith("_id")
        );
        if (uuidField) next[uuidField.name] = id;
      }
      commandFormValues = next;
      syncCommandRawJson();
    } else if (workbenchMode === "queries" && selectedQuery) {
      const fields = extractSchemaFields(selectedQuery.parameters_schema);
      const next = { ...queryFormValues };
      const idField = fields.find((f) => f.name === "id" || f.name.endsWith("id"));
      if (idField) {
        next[idField.name] = id;
      }
      queryFormValues = next;
      syncQueryRawJson();
    }
  }

  function openInWorkbenchFromInspector(sel: InspectorSelection) {
    if (sel.commandRef) {
      activeTab = "workbench";
      selectCommandItem(sel.commandRef);
    } else if (sel.queryRef) {
      activeTab = "workbench";
      selectQueryItem(sel.queryRef);
    }
  }

  function formatContextLabel(slug: string): string {
    return slug
      .split("-")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ");
  }

  function formatTimeShort(iso: string): string {
    if (!iso) return "";
    try {
      const d = new Date(iso);
      return d.toLocaleTimeString([], {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return iso;
    }
  }

  // Group topology per Bounded Context for Canvas view
  let contextTopology = $derived.by(() => {
    return boundedContexts.map((bc) => {
      const commands = catalog.commands.filter((c) => c.bounded_context === bc);
      const aggregates = Array.from(new Set(commands.map((c) => c.target))).sort();
      const readModels = catalog.read_models.filter((rm) => rm.bounded_context === bc);
      const queries = catalog.queries.filter((q) => q.bounded_context === bc);
      return {
        name: bc,
        commands,
        aggregates,
        readModels,
        queries,
      };
    });
  });

  let isMaximized = $state(false);

  async function handleWindowControl(action: "drag" | "minimize" | "toggle_maximize" | "close") {
    if (!isTauri) return;
    try {
      const res: any = await tauriInvoke("window_control", { action });
      if (action === "toggle_maximize") {
        isMaximized = Boolean(res);
      }
    } catch (err) {
      console.warn("Window control failed:", err);
    }
  }

  function handleHeaderMouseDown(e: MouseEvent) {
    if (!isTauri || e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button, a, input, select, textarea, nav")) return;
    if (e.detail === 2) {
      handleWindowControl("toggle_maximize");
    } else {
      handleWindowControl("drag");
    }
  }

  onMount(() => {
    loadCatalogAndEvents();
  });
</script>

<div class="bg-[#F8FAFC] text-on-surface font-body-md antialiased min-h-screen flex flex-col selection:bg-teal-active/20 selection:text-petroleum-void">
  <!-- TOP HEADER BAR (Frameless Draggable Stitch Title Bar) -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header
    onmousedown={handleHeaderMouseDown}
    class="fixed top-0 left-0 right-0 z-50 bg-white/95 backdrop-blur-md border-b border-contour-line-light h-14 select-none cursor-default"
  >
    <div class="max-w-[1840px] h-full mx-auto px-4 lg:px-6 flex items-center justify-between gap-4">
      <!-- Brand & Active Domain Identity -->
      <div class="flex items-center gap-3 shrink-0">
        <div class="flex items-center gap-2">
          <div class="h-7 w-[142px] overflow-hidden flex items-center justify-center">
            <img
              alt="Chromatofoor"
              class="w-[176px] max-w-none h-auto object-cover pointer-events-none mix-blend-multiply"
              src="/logo-horizontal.jpg"
            />
          </div>
          <span class="px-1.5 py-0.5 text-[10px] font-code-mono font-semibold uppercase tracking-wider bg-slate-100 text-slate-600 rounded border border-slate-200">
            Workbench
          </span>
        </div>

        <div class="h-4 w-px bg-slate-200 mx-1"></div>

        <!-- Domain Name & Topic from Backend Catalog -->
        <div class="flex items-center gap-2 px-2.5 py-1 rounded-full bg-slate-100/80 border border-slate-200/80">
          <span class="w-2 h-2 rounded-full {connected ? 'bg-teal-active ring-2 ring-teal-active/20' : 'bg-amber-500'}"></span>
          <span class="font-headline-sm text-[13px] font-semibold tracking-tight text-slate-900">
            {catalog.domain}
          </span>
          <span class="font-code-mono text-[11px] text-slate-500 hidden sm:inline">
            {catalog.topic}
          </span>
        </div>

        <!-- Real Tauri IPC Runtime Connection Pill -->
        <span class="hidden md:inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full {connected ? 'bg-teal-active/10 text-secondary border-teal-active/25' : 'bg-rose-50 text-rose-700 border-rose-200'} text-[11px] font-code-mono font-medium border">
          <span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-teal-active animate-pulse' : 'bg-rose-500'}"></span>
          {connected ? "DomainRuntime Active" : "IPC Disconnected"}
        </span>
      </div>

      <!-- Center Workspace Switcher (Only Real Implemented Views) -->
      <nav class="flex items-center p-1 bg-slate-100/90 rounded-full border border-slate-200/80 shadow-inner">
        <button
          type="button"
          onclick={() => (activeTab = "workbench")}
          class="px-3.5 py-1 rounded-full text-[13px] transition-all flex items-center gap-1.5 cursor-pointer {activeTab === 'workbench'
            ? 'bg-white text-slate-900 shadow-sm font-headline-sm font-semibold'
            : 'text-slate-600 hover:text-slate-900 font-label-md font-medium'}"
        >
          {#if activeTab === "workbench"}
            <span class="w-1.5 h-1.5 rounded-full bg-vermilion-glow"></span>
          {/if}
          <span>Live Workbench &amp; Event Stream</span>
        </button>

        <button
          type="button"
          onclick={() => (activeTab = "canvas")}
          class="px-3.5 py-1 rounded-full text-[13px] transition-all flex items-center gap-1.5 cursor-pointer {activeTab === 'canvas'
            ? 'bg-white text-slate-900 shadow-sm font-headline-sm font-semibold'
            : 'text-slate-600 hover:text-slate-900 font-label-md font-medium'}"
        >
          {#if activeTab === "canvas"}
            <span class="w-1.5 h-1.5 rounded-full bg-teal-active"></span>
          {/if}
          <span>Domain Flow Canvas</span>
        </button>
      </nav>

      <!-- Right: Real Catalog Metrics, Sync Button & Integrated Window Controls -->
      <div class="flex items-center gap-2.5 shrink-0">
        <div class="hidden xl:flex items-center gap-2 text-[11px] font-code-mono text-slate-600 bg-slate-50 px-3 py-1 rounded-full border border-slate-200">
          <span><strong>{boundedContexts.length}</strong> contexts</span>
          <span class="text-slate-300">•</span>
          <span><strong>{catalog.commands.length}</strong> cmds</span>
          <span class="text-slate-300">•</span>
          <span><strong>{catalog.queries.length}</strong> queries</span>
          <span class="text-slate-300">•</span>
          <span class="text-vermilion-glow font-semibold"><strong>{events.length}</strong> events</span>
        </div>

        <button
          type="button"
          onclick={loadCatalogAndEvents}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-white hover:bg-slate-50 text-slate-700 font-label-md text-[12px] font-medium border border-slate-200 transition-colors shadow-sm cursor-pointer"
          title="Reload domain catalog and event stream from DomainRuntime"
        >
          <span class="material-symbols-outlined text-[16px] text-slate-500">refresh</span>
          <span>Sync</span>
        </button>

        {#if isTauri}
          <div class="h-4 w-px bg-slate-200 mx-0.5"></div>

          <!-- Integrated Stitch Window Controls -->
          <div class="flex items-center gap-1">
            <button
              type="button"
              onclick={() => handleWindowControl("minimize")}
              class="w-7 h-7 rounded-full flex items-center justify-center text-slate-500 hover:text-slate-900 hover:bg-slate-100 transition-colors cursor-pointer"
              title="Minimize window"
            >
              <span class="material-symbols-outlined text-[16px]">remove</span>
            </button>
            <button
              type="button"
              onclick={() => handleWindowControl("toggle_maximize")}
              class="w-7 h-7 rounded-full flex items-center justify-center text-slate-500 hover:text-slate-900 hover:bg-slate-100 transition-colors cursor-pointer"
              title={isMaximized ? "Restore window" : "Maximize window"}
            >
              <span class="material-symbols-outlined text-[15px]">
                {isMaximized ? "close_fullscreen" : "open_in_full"}
              </span>
            </button>
            <button
              type="button"
              onclick={() => handleWindowControl("close")}
              class="w-7 h-7 rounded-full flex items-center justify-center text-slate-500 hover:text-white hover:bg-[#cb4831] transition-colors cursor-pointer"
              title="Close Chromatofoor"
            >
              <span class="material-symbols-outlined text-[16px]">close</span>
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if errorBanner}
    <div class="mt-14 bg-rose-50 border-b border-rose-200 px-6 py-2.5 text-rose-800 text-xs font-code-mono flex items-center justify-between">
      <span>{isTauri ? "Tauri Backend Error:" : "HTTP Backend Error:"} {errorBanner}</span>
      <button
        type="button"
        onclick={loadCatalogAndEvents}
        class="px-2.5 py-1 rounded bg-rose-100 hover:bg-rose-200 text-rose-900 font-semibold"
      >
        Retry
      </button>
    </div>
  {/if}

  <!-- ========================================================================= -->
  <!-- VIEW 1: LIVE WORKBENCH & EVENT STREAM (Wired to Commands, Queries, Events)-->
  <!-- ========================================================================= -->
  {#if activeTab === "workbench"}
    <main class="w-full {errorBanner ? '' : 'pt-14'} flex-1 flex flex-col contour-bg">
      <!-- Sub-bar: Real Domain Description & Actor Context -->
      <section class="w-full bg-white/70 backdrop-blur border-b border-contour-line-light px-4 lg:px-8 py-2">
        <div class="max-w-[1840px] mx-auto flex flex-wrap items-center justify-between gap-3 text-[12px]">
          <div class="flex items-center gap-3">
            <span class="font-code-mono text-slate-500 flex items-center gap-1.5">
              <span class="inline-block w-2 h-2 rounded-full bg-teal-active"></span>
              Domain: <strong class="text-slate-800 font-medium">{catalog.domain}</strong>
            </span>
            {#if catalog.description}
              <span class="text-slate-300">•</span>
              <span class="text-slate-600 truncate max-w-xl">{catalog.description}</span>
            {/if}
          </div>

          <!-- Bounded Context Filter Pills -->
          <div class="flex items-center gap-1.5 flex-wrap">
            <span class="text-[11px] font-code-mono text-slate-400 uppercase mr-1">Context:</span>
            <button
              type="button"
              onclick={() => (selectedContext = "all")}
              class="px-2.5 py-0.5 rounded-full text-[11px] font-medium transition-colors {selectedContext === 'all'
                ? 'bg-petroleum-void text-white'
                : 'bg-slate-100 hover:bg-slate-200 text-slate-600'}"
            >
              All ({catalog.commands.length + catalog.queries.length})
            </button>
            {#each boundedContexts as bc}
              <button
                type="button"
                onclick={() => (selectedContext = bc)}
                class="px-2.5 py-0.5 rounded-full text-[11px] font-code-mono transition-colors {selectedContext === bc
                  ? 'bg-petroleum-void text-teal-active font-semibold'
                  : 'bg-slate-100 hover:bg-slate-200 text-slate-600'}"
              >
                {bc}
              </button>
            {/each}
          </div>
        </div>
      </section>

      <!-- Dual-Pane Split Grid -->
      <div class="max-w-[1840px] w-full mx-auto px-4 lg:px-8 py-5 flex-1 grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        <!-- LEFT PANE (7 cols): COMMAND & QUERY WORKBENCH -->
        <section class="lg:col-span-7 flex flex-col gap-5">
          <div class="bg-white rounded-2xl border border-contour-line-light shadow-[0_12px_32px_-8px_rgba(14,35,48,0.07)] overflow-hidden flex flex-col">
            <!-- Top Bar: Mode Switcher (Commands vs Queries) & Actor Selector -->
            <div class="bg-slate-50/90 border-b border-contour-line-light px-5 py-3.5 flex flex-wrap items-center justify-between gap-3">
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  onclick={() => (workbenchMode = "commands")}
                  class="px-3.5 py-1.5 rounded-xl text-xs font-headline-sm font-semibold transition-all flex items-center gap-1.5 {workbenchMode === 'commands'
                    ? 'bg-blue-600 text-white shadow-sm'
                    : 'bg-white text-slate-600 hover:text-slate-900 border border-slate-200'}"
                >
                  <span class="material-symbols-outlined text-[15px]">bolt</span>
                  <span>Commands ({filteredCommands.length})</span>
                </button>

                <button
                  type="button"
                  onclick={() => (workbenchMode = "queries")}
                  class="px-3.5 py-1.5 rounded-xl text-xs font-headline-sm font-semibold transition-all flex items-center gap-1.5 {workbenchMode === 'queries'
                    ? 'bg-emerald-600 text-white shadow-sm'
                    : 'bg-white text-slate-600 hover:text-slate-900 border border-slate-200'}"
                >
                  <span class="material-symbols-outlined text-[15px]">database</span>
                  <span>Queries &amp; Read Models ({filteredQueries.length})</span>
                </button>
              </div>

              <!-- Real Actor Selector from Catalog -->
              <div class="flex items-center gap-1.5 bg-white p-1 rounded-full border border-slate-200/90 shadow-2xs">
                <span class="text-[11px] font-code-mono text-slate-400 pl-2.5 pr-1">Actor:</span>
                {#each catalog.actors as actor}
                  <button
                    type="button"
                    onclick={() => (activeActor = actor)}
                    class="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all {activeActor === actor
                      ? 'bg-petroleum-void text-white shadow-xs'
                      : 'text-slate-600 hover:text-slate-900'}"
                  >
                    <span class="material-symbols-outlined text-[14px] {activeActor === actor ? 'text-teal-active' : 'text-slate-400'}">person</span>
                    <span>{actor}</span>
                  </button>
                {/each}
              </div>
            </div>

            <!-- Main Workbench Body -->
            <div class="p-6 topo-lines grid grid-cols-1 md:grid-cols-12 gap-6">
              <!-- Left Column (5 cols): Registered Commands or Queries List -->
              <div class="md:col-span-5 flex flex-col gap-2.5 border-r border-slate-200/80 pr-0 md:pr-4 max-h-[600px] overflow-y-auto">
                {#if workbenchMode === "commands"}
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-[11px] font-code-mono uppercase tracking-wider text-slate-400 font-semibold">
                      Registered Commands
                    </span>
                    <span class="text-[11px] font-code-mono text-blue-600 font-medium">
                      Write Side
                    </span>
                  </div>

                  {#each filteredCommands as cmd}
                    {@const isSelected =
                      selectedCommand?.name === cmd.name &&
                      selectedCommand?.bounded_context === cmd.bounded_context}
                    <button
                      type="button"
                      onclick={() => selectCommandItem(cmd)}
                      class="w-full text-left p-3 rounded-xl border transition-all flex flex-col gap-1 {isSelected
                        ? 'bg-blue-50/70 border-blue-500 ring-2 ring-blue-500/15 shadow-xs'
                        : 'bg-white/90 hover:bg-slate-50 border-slate-200/90'}"
                    >
                      <div class="flex items-center justify-between gap-2">
                        <span class="font-headline-sm text-[13px] font-bold text-slate-900 truncate">
                          {cmd.name}
                        </span>
                        <span class="px-1.5 py-0.5 rounded text-[10px] font-code-mono font-medium bg-amber-50 text-amber-800 border border-amber-200/80 shrink-0">
                          {cmd.target}
                        </span>
                      </div>
                      <p class="text-[11px] text-slate-500 line-clamp-2 leading-relaxed">
                        {cmd.description || cmd.topic}
                      </p>
                      <div class="flex items-center gap-1.5 mt-1">
                        <span class="text-[10px] font-code-mono text-slate-400">
                          {cmd.bounded_context}
                        </span>
                      </div>
                    </button>
                  {/each}
                {:else}
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-[11px] font-code-mono uppercase tracking-wider text-slate-400 font-semibold">
                      Registered Queries
                    </span>
                    <span class="text-[11px] font-code-mono text-emerald-600 font-medium">
                      Read Side
                    </span>
                  </div>

                  {#each filteredQueries as q}
                    {@const isSelected =
                      selectedQuery?.name === q.name &&
                      selectedQuery?.bounded_context === q.bounded_context}
                    <button
                      type="button"
                      onclick={() => selectQueryItem(q)}
                      class="w-full text-left p-3 rounded-xl border transition-all flex flex-col gap-1 {isSelected
                        ? 'bg-emerald-50/70 border-emerald-500 ring-2 ring-emerald-500/15 shadow-xs'
                        : 'bg-white/90 hover:bg-slate-50 border-slate-200/90'}"
                    >
                      <div class="flex items-center justify-between gap-2">
                        <span class="font-headline-sm text-[13px] font-bold text-slate-900 truncate">
                          {q.name}
                        </span>
                        <span class="px-1.5 py-0.5 rounded text-[10px] font-code-mono font-medium bg-emerald-50 text-emerald-800 border border-emerald-200/80 shrink-0">
                          {q.read_model}
                        </span>
                      </div>
                      <p class="text-[11px] text-slate-500 line-clamp-2 leading-relaxed">
                        {q.description || q.topic}
                      </p>
                      <div class="flex items-center gap-1.5 mt-1">
                        <span class="text-[10px] font-code-mono text-slate-400">
                          {q.bounded_context}
                        </span>
                      </div>
                    </button>
                  {/each}
                {/if}
              </div>

              <!-- Right Column (7 cols): Schema-Driven Form & Execution Output -->
              <div class="md:col-span-7 flex flex-col gap-4">
                {#if workbenchMode === "commands" && selectedCommand}
                  {@const fields = extractSchemaFields(selectedCommand.schema)}
                  <!-- Selected Command Header -->
                  <div class="bg-slate-50/90 rounded-xl p-4 border border-slate-200/90">
                    <div class="flex items-start justify-between gap-3">
                      <div>
                        <div class="flex items-center gap-2 flex-wrap">
                          <span class="px-2 py-0.5 rounded text-[10px] font-code-mono font-semibold uppercase bg-blue-100 text-blue-800">
                            Command
                          </span>
                          <span class="text-xs font-code-mono text-slate-500">
                            {selectedCommand.bounded_context} / <strong class="text-amber-700">{selectedCommand.target}</strong>
                          </span>
                        </div>
                        <h2 class="font-headline-md text-lg font-bold text-slate-900 mt-1">
                          {selectedCommand.name}
                        </h2>
                        <p class="text-xs text-slate-600 mt-0.5">
                          {selectedCommand.description}
                        </p>
                      </div>

                      <button
                        type="button"
                        onclick={() => {
                          syncCommandRawJson();
                          rawJsonMode = !rawJsonMode;
                        }}
                        class="px-2.5 py-1 rounded-lg text-[11px] font-code-mono border transition-colors shrink-0 {rawJsonMode
                          ? 'bg-petroleum-void text-teal-active border-petroleum-void'
                          : 'bg-white text-slate-600 border-slate-200 hover:bg-slate-100'}"
                      >
                        {rawJsonMode ? "Form View" : "Raw JSON"}
                      </button>
                    </div>
                    <div class="mt-2.5 pt-2.5 border-t border-slate-200/70 flex items-center justify-between text-[11px] font-code-mono text-slate-500">
                      <span>Topic: <code class="text-slate-700">{selectedCommand.topic}</code></span>
                      <span>Actors: <strong class="text-slate-700">{selectedCommand.allowed_actors.join(", ") || "any"}</strong></span>
                    </div>
                  </div>

                  <!-- Known Aggregate Instances Bar (for quick ID injection) -->
                  {#if knownInstances.length > 0}
                    <div class="bg-amber-50/60 border border-amber-200/80 rounded-xl p-3">
                      <div class="text-[11px] font-code-mono text-amber-900 font-semibold mb-1.5 flex items-center gap-1.5">
                        <span class="material-symbols-outlined text-[14px] text-amber-600">layers</span>
                        <span>Active Aggregate Instances (Click to fill ID field):</span>
                      </div>
                      <div class="flex flex-wrap gap-1.5">
                        {#each knownInstances as inst}
                          <button
                            type="button"
                            onclick={() => applyInstanceIdToActiveForm(inst.id, inst.aggregate)}
                            class="px-2.5 py-1 rounded-lg bg-white hover:bg-amber-100/70 text-slate-800 border border-amber-300/80 text-[11px] font-code-mono flex items-center gap-1.5 transition-colors shadow-2xs"
                            title="Aggregate: {inst.aggregate} | UUID: {inst.id}"
                          >
                            <span class="font-semibold text-amber-800">{inst.aggregate}:</span>
                            <span>{inst.label}</span>
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/if}

                  <!-- Schema Form or Raw JSON -->
                  {#if rawJsonMode}
                    <div class="flex flex-col gap-1.5">
                      <label for="raw-cmd-json" class="text-xs font-semibold text-slate-700 font-code-mono">
                        Command Payload (JSON)
                      </label>
                      <textarea
                        id="raw-cmd-json"
                        bind:value={rawCommandJson}
                        rows="8"
                        class="w-full rounded-xl border border-slate-300 bg-slate-900 text-teal-active font-code-mono text-xs p-3.5 focus:ring-2 focus:ring-teal-active focus:border-teal-active"
                      ></textarea>
                    </div>
                  {:else}
                    <div class="flex flex-col gap-3.5">
                      {#each fields as field}
                        <div class="flex flex-col gap-1">
                          <div class="flex items-center justify-between">
                            <label for="cmd-field-{field.name}" class="text-xs font-semibold text-slate-700 flex items-center gap-1.5">
                              <span>{field.name}</span>
                              {#if field.required}
                                <span class="text-vermilion-glow">*</span>
                              {:else}
                                <span class="text-[10px] font-code-mono text-slate-400">(optional)</span>
                              {/if}
                            </label>
                            <div class="flex items-center gap-2">
                              <span class="text-[10px] font-code-mono text-slate-400">
                                {field.format ? `${field.type} (${field.format})` : field.type}
                              </span>
                              {#if field.format === "uuid" || field.name === "id" || field.name.endsWith("-id")}
                                <button
                                  type="button"
                                  onclick={() => {
                                    commandFormValues = {
                                      ...commandFormValues,
                                      [field.name]: generateUuid(),
                                    };
                                    syncCommandRawJson();
                                  }}
                                  class="text-[10px] font-code-mono text-blue-600 hover:text-blue-800 underline"
                                >
                                  New UUID
                                </button>
                              {/if}
                            </div>
                          </div>

                          {#if field.enumValues}
                            <select
                              id="cmd-field-{field.name}"
                              bind:value={commandFormValues[field.name]}
                              onchange={syncCommandRawJson}
                              class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-code-mono text-slate-800 focus:ring-2 focus:ring-blue-500/20 focus:border-blue-600"
                            >
                              {#each field.enumValues as val}
                                <option value={val}>{val}</option>
                              {/each}
                            </select>
                          {:else if field.type === "boolean"}
                            <label class="inline-flex items-center gap-2 text-xs text-slate-700">
                              <input
                                id="cmd-field-{field.name}"
                                type="checkbox"
                                bind:checked={commandFormValues[field.name]}
                                onchange={syncCommandRawJson}
                                class="rounded border-slate-300 text-blue-600 focus:ring-blue-500"
                              />
                              <span>Enabled</span>
                            </label>
                          {:else if field.name.includes("yaml") || field.name.includes("snippet") || field.name.includes("prompt")}
                            <textarea
                              id="cmd-field-{field.name}"
                              bind:value={commandFormValues[field.name]}
                              oninput={syncCommandRawJson}
                              rows="3"
                              class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-code-mono text-slate-800 focus:ring-2 focus:ring-blue-500/20 focus:border-blue-600"
                            ></textarea>
                          {:else}
                            <input
                              id="cmd-field-{field.name}"
                              type={field.type === "integer" || field.type === "number" ? "number" : "text"}
                              bind:value={commandFormValues[field.name]}
                              oninput={syncCommandRawJson}
                              class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-code-mono text-slate-800 focus:ring-2 focus:ring-blue-500/20 focus:border-blue-600"
                            />
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {/if}

                  <!-- Terracotta Dispatch CTA Button -->
                  <button
                    type="button"
                    disabled={isSubmitting}
                    onclick={handleDispatchCommand}
                    class="w-full py-3 px-5 rounded-xl bg-gradient-to-r from-[#a9301b] to-[#cb4831] hover:from-[#922815] hover:to-[#b83e28] disabled:opacity-50 text-white font-headline-sm text-sm font-semibold shadow-md shadow-primary/15 flex items-center justify-center gap-2 transition-all cursor-pointer mt-1"
                  >
                    <span class="material-symbols-outlined text-[18px]">bolt</span>
                    <span>
                      {isSubmitting ? "Dispatching..." : `Dispatch ${selectedCommand.name}`}
                    </span>
                  </button>

                  <!-- Command Result Output -->
                  {#if commandResult}
                    <div class="rounded-xl p-4 border {commandResult.ok ? 'bg-emerald-50/70 border-emerald-200' : 'bg-rose-50 border-rose-200'}">
                      <div class="flex items-center justify-between gap-2 mb-2">
                        <div class="flex items-center gap-1.5">
                          <span class="material-symbols-outlined text-[18px] {commandResult.ok ? 'text-emerald-600' : 'text-rose-600'}">
                            {commandResult.ok ? "check_circle" : "error"}
                          </span>
                          <span class="font-headline-sm text-xs font-bold {commandResult.ok ? 'text-emerald-950' : 'text-rose-950'}">
                            {commandResult.ok
                              ? `Dispatched ${commandResult.command} (${commandResult.emittedEvents.length} event${commandResult.emittedEvents.length === 1 ? "" : "s"} emitted)`
                              : `Command Rejected: ${commandResult.command}`}
                          </span>
                        </div>
                      </div>

                      {#if commandResult.error}
                        <pre class="text-xs font-code-mono text-rose-800 whitespace-pre-wrap bg-white/80 p-2.5 rounded-lg border border-rose-200/80">{commandResult.error}</pre>
                      {/if}

                      {#if commandResult.emittedEvents.length > 0}
                        {#each commandResult.emittedEvents as ev}
                          <div class="bg-white rounded-lg p-3 border border-emerald-200/90 mt-2 flex flex-col gap-1.5">
                            <div class="flex items-center justify-between gap-2 flex-wrap">
                              <span class="px-2 py-0.5 rounded text-[11px] font-code-mono font-semibold bg-vermilion-glow/15 text-primary border border-vermilion-glow/30">
                                {ev.event_name}
                              </span>
                              <span class="text-[11px] font-code-mono text-slate-600">
                                Instance UUID: <strong class="text-slate-900 select-all">{ev.instance_id}</strong>
                              </span>
                            </div>
                            <pre class="text-[11px] font-code-mono text-slate-700 bg-slate-50 p-2 rounded border border-slate-200/70 overflow-x-auto">{JSON.stringify(ev.data, null, 2)}</pre>
                            <div class="flex items-center justify-end gap-2 mt-1">
                              <button
                                type="button"
                                onclick={() => jumpToQueryWithInstance(commandResult!.bounded_context, ev.instance_id)}
                                class="px-2.5 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white text-[11px] font-code-mono font-medium flex items-center gap-1 transition-colors"
                              >
                                <span class="material-symbols-outlined text-[14px]">database</span>
                                <span>Query Read Model ({ev.instance_id.slice(0, 8)}…)</span>
                              </button>
                            </div>
                          </div>
                        {/each}
                      {/if}
                    </div>
                  {/if}
                {:else if workbenchMode === "queries" && selectedQuery}
                  {@const fields = extractSchemaFields(selectedQuery.parameters_schema)}
                  <!-- Selected Query Header -->
                  <div class="bg-slate-50/90 rounded-xl p-4 border border-slate-200/90">
                    <div class="flex items-start justify-between gap-3">
                      <div>
                        <div class="flex items-center gap-2 flex-wrap">
                          <span class="px-2 py-0.5 rounded text-[10px] font-code-mono font-semibold uppercase bg-emerald-100 text-emerald-800">
                            Query
                          </span>
                          <span class="text-xs font-code-mono text-slate-500">
                            {selectedQuery.bounded_context} / Read Model: <strong class="text-emerald-700">{selectedQuery.read_model}</strong>
                          </span>
                        </div>
                        <h2 class="font-headline-md text-lg font-bold text-slate-900 mt-1">
                          {selectedQuery.name}
                        </h2>
                        <p class="text-xs text-slate-600 mt-0.5">
                          {selectedQuery.description}
                        </p>
                      </div>

                      <button
                        type="button"
                        onclick={() => {
                          syncQueryRawJson();
                          rawJsonMode = !rawJsonMode;
                        }}
                        class="px-2.5 py-1 rounded-lg text-[11px] font-code-mono border transition-colors shrink-0 {rawJsonMode
                          ? 'bg-petroleum-void text-teal-active border-petroleum-void'
                          : 'bg-white text-slate-600 border-slate-200 hover:bg-slate-100'}"
                      >
                        {rawJsonMode ? "Form View" : "Raw JSON"}
                      </button>
                    </div>
                    <div class="mt-2.5 pt-2.5 border-t border-slate-200/70 flex items-center justify-between text-[11px] font-code-mono text-slate-500">
                      <span>Topic: <code class="text-slate-700">{selectedQuery.topic}</code></span>
                      <span>Actors: <strong class="text-slate-700">{selectedQuery.allowed_actors.join(", ") || "any"}</strong></span>
                    </div>
                  </div>

                  <!-- Known Aggregate Instances Bar (for quick Query ID selection) -->
                  {#if knownInstances.length > 0}
                    <div class="bg-emerald-50/60 border border-emerald-200/80 rounded-xl p-3">
                      <div class="text-[11px] font-code-mono text-emerald-900 font-semibold mb-1.5 flex items-center gap-1.5">
                        <span class="material-symbols-outlined text-[14px] text-emerald-600">database</span>
                        <span>Projected Instances in Store (Click to query):</span>
                      </div>
                      <div class="flex flex-wrap gap-1.5">
                        {#each knownInstances as inst}
                          <button
                            type="button"
                            onclick={() => {
                              applyInstanceIdToActiveForm(inst.id, inst.aggregate);
                              handleExecuteQuery();
                            }}
                            class="px-2.5 py-1 rounded-lg bg-white hover:bg-emerald-100/70 text-slate-800 border border-emerald-300/80 text-[11px] font-code-mono flex items-center gap-1.5 transition-colors shadow-2xs"
                          >
                            <span class="font-semibold text-emerald-800">{inst.aggregate}:</span>
                            <span>{inst.label}</span>
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/if}

                  {#if rawJsonMode}
                    <div class="flex flex-col gap-1.5">
                      <label for="raw-query-json" class="text-xs font-semibold text-slate-700 font-code-mono">
                        Query Parameters (JSON)
                      </label>
                      <textarea
                        id="raw-query-json"
                        bind:value={rawQueryJson}
                        rows="5"
                        class="w-full rounded-xl border border-slate-300 bg-slate-900 text-teal-active font-code-mono text-xs p-3.5 focus:ring-2 focus:ring-teal-active focus:border-teal-active"
                      ></textarea>
                    </div>
                  {:else}
                    <div class="flex flex-col gap-3.5">
                      {#each fields as field}
                        <div class="flex flex-col gap-1">
                          <div class="flex items-center justify-between">
                            <label for="query-field-{field.name}" class="text-xs font-semibold text-slate-700">
                              {field.name}
                              {#if field.required}
                                <span class="text-vermilion-glow">*</span>
                              {/if}
                            </label>
                            <span class="text-[10px] font-code-mono text-slate-400">
                              {field.type}
                            </span>
                          </div>
                          <input
                            id="query-field-{field.name}"
                            type="text"
                            bind:value={queryFormValues[field.name]}
                            oninput={syncQueryRawJson}
                            placeholder="Aggregate instance UUID..."
                            class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-code-mono text-slate-800 focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600"
                          />
                        </div>
                      {/each}
                    </div>
                  {/if}

                  <button
                    type="button"
                    disabled={isSubmitting}
                    onclick={handleExecuteQuery}
                    class="w-full py-3 px-5 rounded-xl bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-headline-sm text-sm font-semibold shadow-md shadow-emerald-600/15 flex items-center justify-center gap-2 transition-all cursor-pointer mt-1"
                  >
                    <span class="material-symbols-outlined text-[18px]">manage_search</span>
                    <span>
                      {isSubmitting ? "Querying Read Model..." : `Execute ${selectedQuery.name}`}
                    </span>
                  </button>

                  <!-- Query Result Output -->
                  {#if queryResult}
                    <div class="rounded-xl p-4 border {queryResult.ok ? 'bg-emerald-50/60 border-emerald-200' : 'bg-rose-50 border-rose-200'}">
                      <div class="flex items-center justify-between mb-2">
                        <div class="flex items-center gap-1.5">
                          <span class="material-symbols-outlined text-[18px] {queryResult.ok ? 'text-emerald-600' : 'text-rose-600'}">
                            {queryResult.ok ? "dataset" : "error"}
                          </span>
                          <span class="font-headline-sm text-xs font-bold {queryResult.ok ? 'text-emerald-950' : 'text-rose-950'}">
                            {queryResult.ok
                              ? `Projected Read Model: ${queryResult.read_model}`
                              : `Query Failed: ${queryResult.query}`}
                          </span>
                        </div>
                      </div>

                      {#if queryResult.ok}
                        <pre class="text-xs font-code-mono text-slate-800 bg-white p-3.5 rounded-xl border border-emerald-200/80 overflow-x-auto">{JSON.stringify(queryResult.data, null, 2)}</pre>
                      {:else}
                        <pre class="text-xs font-code-mono text-rose-800 whitespace-pre-wrap bg-white/80 p-3 rounded-xl border border-rose-200/80">{queryResult.error}</pre>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>
            </div>
          </div>
        </section>

        <!-- RIGHT PANE (5 cols): DEEP PETROLEUM REAL-TIME EVENT STREAM -->
        <aside class="lg:col-span-5 flex flex-col bg-petroleum-void text-white rounded-2xl border border-contour-line-dark shadow-[0_20px_50px_rgba(14,35,48,0.22)] overflow-hidden petroleum-topo min-h-[640px]">
          <!-- Header -->
          <div class="px-5 py-4 border-b border-contour-line-dark bg-petroleum-surface/80 flex items-center justify-between gap-3">
            <div class="flex items-center gap-2.5">
              <span class="w-2.5 h-2.5 rounded-full bg-vermilion-glow animate-pulse"></span>
              <div>
                <h2 class="font-headline-sm text-sm font-bold tracking-tight text-white">
                  Live Domain Event Stream
                </h2>
                <p class="text-[11px] font-code-mono text-slate-400">
                  Store Topic: {catalog.topic}
                </p>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <span class="px-2.5 py-0.5 rounded-full text-[11px] font-code-mono bg-petroleum-elevated text-teal-active border border-contour-line-dark">
                {events.length} {events.length === 1 ? "event" : "events"}
              </span>
              <button
                type="button"
                onclick={refreshEvents}
                class="p-1.5 rounded-lg bg-petroleum-elevated hover:bg-slate-800 text-slate-300 hover:text-white border border-contour-line-dark transition-colors"
                title="Refresh Event Stream"
              >
                <span class="material-symbols-outlined text-[16px]">refresh</span>
              </button>
            </div>
          </div>

          <!-- Filter Bar -->
          <div class="px-5 py-2.5 border-b border-contour-line-dark bg-petroleum-void/60 flex items-center gap-2">
            <span class="material-symbols-outlined text-[16px] text-slate-400">filter_alt</span>
            <input
              type="text"
              bind:value={eventFilter}
              placeholder="Filter by event name, subject, UUID, or payload..."
              class="w-full bg-transparent border-0 text-xs font-code-mono text-white placeholder-slate-500 focus:ring-0 p-0"
            />
            {#if eventFilter}
              <button
                type="button"
                onclick={() => (eventFilter = "")}
                class="text-[11px] font-code-mono text-slate-400 hover:text-white"
              >
                Clear
              </button>
            {/if}
          </div>

          <!-- Event Stream Feed -->
          <div class="p-4 flex-1 flex flex-col gap-3 overflow-y-auto max-h-[600px]">
            {#if displayedEvents.length === 0}
              <div class="flex-1 flex flex-col items-center justify-center text-center py-16 px-6">
                <div class="w-12 h-12 rounded-2xl bg-petroleum-elevated border border-contour-line-dark flex items-center justify-center mb-3">
                  <span class="material-symbols-outlined text-teal-active text-[24px]">sensors</span>
                </div>
                <p class="font-headline-sm text-sm font-semibold text-slate-200">
                  {events.length === 0 ? "No Domain Events Emitted Yet" : "No Matching Events"}
                </p>
                <p class="text-xs text-slate-400 mt-1 max-w-xs leading-relaxed">
                  {events.length === 0
                    ? "Dispatch any Command on the left workbench to persist domain events into the Store and watch them stream here in real time."
                    : "Try clearing the filter above to see all stored domain events."}
                </p>
              </div>
            {:else}
              {#each displayedEvents as ev}
                <div class="rounded-xl bg-petroleum-surface/90 border border-contour-line-dark p-3.5 hover:border-teal-active/40 transition-all flex flex-col gap-2">
                  <div class="flex items-center justify-between gap-2 flex-wrap">
                    <div class="flex items-center gap-2">
                      <span class="px-2 py-0.5 rounded text-[11px] font-code-mono font-semibold bg-vermilion-glow/20 text-[#ff855e] border border-vermilion-glow/30">
                        {ev.event_name}
                      </span>
                      <span class="text-[11px] font-code-mono text-slate-400">
                        #{ev.id}
                      </span>
                    </div>
                    <span class="text-[11px] font-code-mono text-slate-400">
                      {formatTimeShort(ev.time)}
                    </span>
                  </div>

                  <!-- Subject & Clickable Instance ID -->
                  <div class="flex items-center justify-between gap-2 text-[11px] font-code-mono bg-petroleum-void/80 px-2.5 py-1.5 rounded-lg border border-contour-line-dark">
                    <span class="text-slate-300 truncate" title={ev.subject}>
                      {ev.subject}
                    </span>
                    <button
                      type="button"
                      onclick={() => applyInstanceIdToActiveForm(ev.instance_id)}
                      class="text-teal-active hover:underline shrink-0 font-semibold"
                      title="Click to use this instance UUID in the active form"
                    >
                      Use ID
                    </button>
                  </div>

                  <!-- Event Payload JSON -->
                  <pre class="text-[11px] font-code-mono text-slate-200 bg-petroleum-void/60 p-2.5 rounded-lg border border-contour-line-dark/80 overflow-x-auto leading-relaxed">{JSON.stringify(ev.data, null, 2)}</pre>
                </div>
              {/each}
            {/if}
          </div>
        </aside>
      </div>
    </main>

  <!-- ========================================================================= -->
  <!-- VIEW 2: DOMAIN FLOW CANVAS (Visualizing Real Bounded Contexts & Schema)   -->
  <!-- ========================================================================= -->
  {:else if activeTab === "canvas"}
    <main class="w-full {errorBanner ? '' : 'pt-14'} flex-1 flex overflow-hidden contour-bg">
      <!-- Canvas Area -->
      <div class="flex-1 p-6 lg:p-8 overflow-y-auto flex flex-col gap-6">
        <!-- Legend & Summary Bar -->
        <div class="bg-white/90 backdrop-blur rounded-2xl border border-contour-line-light px-5 py-3.5 shadow-2xs flex flex-wrap items-center justify-between gap-4">
          <div>
            <h1 class="font-headline-sm text-base font-bold text-slate-900">
              Domain Architecture Topology
            </h1>
            <p class="text-xs text-slate-500">
              Live structural introspection from <code class="font-code-mono text-slate-700">DomainRuntime::catalog_json()</code>. Click any element to inspect its schema or execute it.
            </p>
          </div>

          <div class="flex items-center gap-4 text-xs font-medium">
            <span class="flex items-center gap-1.5 text-slate-700">
              <span class="w-2.5 h-2.5 rounded-sm bg-blue-600"></span> Command
            </span>
            <span class="flex items-center gap-1.5 text-slate-700">
              <span class="w-2.5 h-2.5 rounded-sm bg-amber-500"></span> Aggregate
            </span>
            <span class="flex items-center gap-1.5 text-slate-700">
              <span class="w-2.5 h-2.5 rounded-sm bg-emerald-600"></span> Read Model / Query
            </span>
            <span class="flex items-center gap-1.5 text-slate-700">
              <span class="w-2.5 h-2.5 rounded-sm bg-purple-600"></span> Event Handler
            </span>
          </div>
        </div>

        <!-- Bounded Context Frames Grid -->
        <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
          {#each contextTopology as ctx}
            <section class="bg-slate-50/85 rounded-2xl border border-slate-200/90 p-5 shadow-xs flex flex-col gap-4">
              <!-- Bounded Context Frame Header -->
              <div class="flex items-center justify-between border-b border-slate-200 pb-3">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-teal-active text-[18px]">hub</span>
                  <h2 class="font-headline-sm text-sm font-bold text-slate-900 uppercase tracking-wider">
                    {formatContextLabel(ctx.name)}
                  </h2>
                  <span class="font-code-mono text-[11px] text-slate-400">
                    ({ctx.name})
                  </span>
                </div>
                <span class="px-2.5 py-0.5 rounded-full text-[11px] font-code-mono bg-white text-slate-600 border border-slate-200">
                  {ctx.commands.length} cmds • {ctx.queries.length} queries
                </span>
              </div>

              <!-- 3-Column Flow inside Bounded Context: Commands -> Aggregates -> Read Models & Queries -->
              <div class="grid grid-cols-1 md:grid-cols-3 gap-3 items-start">
                <!-- Column 1: Commands -->
                <div class="flex flex-col gap-2">
                  <span class="text-[10px] font-code-mono uppercase tracking-wider text-blue-700 font-semibold">
                    Commands ({ctx.commands.length})
                  </span>
                  {#each ctx.commands as cmd}
                    <button
                      type="button"
                      onclick={() =>
                        (inspectorItem = {
                          kind: "command",
                          bounded_context: cmd.bounded_context,
                          name: cmd.name,
                          target: cmd.target,
                          description: cmd.description,
                          topic: cmd.topic,
                          allowed_actors: cmd.allowed_actors,
                          schema: cmd.schema,
                          commandRef: cmd,
                        })}
                      class="text-left p-2.5 rounded-xl bg-white hover:bg-blue-50/50 border-l-4 border-l-blue-600 border border-slate-200/90 shadow-2xs transition-all"
                    >
                      <div class="font-headline-sm text-xs font-bold text-slate-900 truncate">
                        {cmd.name}
                      </div>
                      <div class="text-[10px] font-code-mono text-slate-400 mt-0.5 truncate">
                        → {cmd.target}
                      </div>
                    </button>
                  {/each}
                </div>

                <!-- Column 2: Aggregates -->
                <div class="flex flex-col gap-2">
                  <span class="text-[10px] font-code-mono uppercase tracking-wider text-amber-700 font-semibold">
                    Aggregates ({ctx.aggregates.length})
                  </span>
                  {#each ctx.aggregates as agg}
                    {@const aggCmds = ctx.commands.filter((c) => c.target === agg)}
                    <button
                      type="button"
                      onclick={() =>
                        (inspectorItem = {
                          kind: "aggregate",
                          bounded_context: ctx.name,
                          name: agg,
                          description: `Consistency boundary in ${ctx.name} handling ${aggCmds.length} command(s).`,
                          topic: `${catalog.topic}/${ctx.name}/${agg}`,
                        })}
                      class="text-left p-3 rounded-xl bg-white hover:bg-amber-50/50 border-l-4 border-l-amber-500 border border-slate-200/90 shadow-2xs transition-all"
                    >
                      <div class="font-headline-sm text-xs font-bold text-slate-900">
                        {agg}
                      </div>
                      <div class="text-[10px] font-code-mono text-amber-700 mt-1">
                        {aggCmds.length} command{aggCmds.length === 1 ? "" : "s"}
                      </div>
                    </button>
                  {/each}
                </div>

                <!-- Column 3: Read Models & Queries -->
                <div class="flex flex-col gap-2">
                  <span class="text-[10px] font-code-mono uppercase tracking-wider text-emerald-700 font-semibold">
                    Read Models &amp; Queries
                  </span>
                  {#each ctx.readModels as rm}
                    <button
                      type="button"
                      onclick={() =>
                        (inspectorItem = {
                          kind: "read-model",
                          bounded_context: rm.bounded_context,
                          name: rm.name,
                          description: rm.description,
                          topic: rm.topic,
                          events: rm.events,
                        })}
                      class="text-left p-2.5 rounded-xl bg-white hover:bg-emerald-50/50 border-l-4 border-l-emerald-600 border border-slate-200/90 shadow-2xs transition-all"
                    >
                      <div class="flex items-center justify-between">
                        <span class="font-headline-sm text-xs font-bold text-slate-900 truncate">
                          {rm.name}
                        </span>
                        <span class="text-[9px] font-code-mono uppercase px-1.5 py-0.5 rounded bg-emerald-50 text-emerald-700">
                          View
                        </span>
                      </div>
                      <div class="text-[10px] font-code-mono text-slate-400 mt-0.5">
                        {rm.events.length} projected event{rm.events.length === 1 ? "" : "s"}
                      </div>
                    </button>
                  {/each}

                  {#each ctx.queries as q}
                    <button
                      type="button"
                      onclick={() =>
                        (inspectorItem = {
                          kind: "query",
                          bounded_context: q.bounded_context,
                          name: q.name,
                          target: q.read_model,
                          description: q.description,
                          topic: q.topic,
                          allowed_actors: q.allowed_actors,
                          schema: q.parameters_schema,
                          output_schema: q.output_schema,
                          queryRef: q,
                        })}
                      class="text-left p-2.5 rounded-xl bg-white hover:bg-emerald-50/50 border-l-4 border-l-teal-500 border border-slate-200/90 shadow-2xs transition-all"
                    >
                      <div class="flex items-center justify-between">
                        <span class="font-headline-sm text-xs font-bold text-slate-900 truncate">
                          {q.name}
                        </span>
                        <span class="text-[9px] font-code-mono uppercase px-1.5 py-0.5 rounded bg-teal-50 text-teal-800">
                          Query
                        </span>
                      </div>
                      <div class="text-[10px] font-code-mono text-slate-400 mt-0.5 truncate">
                        reads {q.read_model}
                      </div>
                    </button>
                  {/each}
                </div>
              </div>
            </section>
          {/each}
        </div>

        <!-- Domain-Level Event Consumers Section -->
        {#if catalog.consumers.length > 0}
          <section class="bg-white rounded-2xl border border-contour-line-light p-5 shadow-2xs">
            <div class="flex items-center gap-2 mb-3">
              <span class="material-symbols-outlined text-purple-600 text-[18px]">precision_manufacturing</span>
              <h2 class="font-headline-sm text-sm font-bold text-slate-900 uppercase tracking-wider">
                Registered Event Consumers ({catalog.consumers.length})
              </h2>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {#each catalog.consumers as consumer}
                <button
                  type="button"
                  onclick={() =>
                    (inspectorItem = {
                      kind: "consumer",
                      bounded_context: "integration",
                      name: consumer.name,
                      description: consumer.description,
                      topic: consumer.topic,
                    })}
                  class="text-left p-3.5 rounded-xl bg-slate-50/80 hover:bg-purple-50/50 border-l-4 border-l-purple-600 border border-slate-200 transition-all"
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="font-headline-sm text-xs font-bold text-slate-900">
                      {consumer.name}
                    </span>
                    <span class="px-2 py-0.5 rounded text-[10px] font-code-mono bg-purple-100 text-purple-800">
                      {consumer.kind}
                    </span>
                  </div>
                  <p class="text-xs text-slate-600 mt-1">
                    {consumer.description || consumer.topic}
                  </p>
                </button>
              {/each}
            </div>
          </section>
        {/if}
      </div>

      <!-- Right Contextual Inspector Drawer -->
      <aside class="w-96 bg-white border-l border-contour-line-light p-5 overflow-y-auto flex flex-col gap-4 shrink-0">
        {#if inspectorItem}
          <div class="flex items-start justify-between gap-2 border-b border-slate-200 pb-3">
            <div>
              <span class="px-2 py-0.5 rounded text-[10px] font-code-mono uppercase font-semibold bg-slate-100 text-slate-700">
                {inspectorItem.kind}
              </span>
              <h3 class="font-headline-md text-base font-bold text-slate-900 mt-1">
                {inspectorItem.name}
              </h3>
              <p class="text-xs font-code-mono text-slate-500">
                {inspectorItem.bounded_context}
              </p>
            </div>
            <button
              type="button"
              onclick={() => (inspectorItem = null)}
              class="text-slate-400 hover:text-slate-700"
            >
              <span class="material-symbols-outlined text-[18px]">close</span>
            </button>
          </div>

          <p class="text-xs text-slate-600 leading-relaxed">
            {inspectorItem.description}
          </p>

          <div class="bg-slate-50 rounded-xl p-3 border border-slate-200/80 text-xs font-code-mono flex flex-col gap-1.5">
            <div>
              <span class="text-slate-400">Topic:</span>
              <div class="text-slate-800 break-all">{inspectorItem.topic}</div>
            </div>
            {#if inspectorItem.allowed_actors && inspectorItem.allowed_actors.length > 0}
              <div>
                <span class="text-slate-400">Allowed Actors:</span>
                <span class="text-slate-800 font-semibold">{inspectorItem.allowed_actors.join(", ")}</span>
              </div>
            {/if}
          </div>

          {#if inspectorItem.events && inspectorItem.events.length > 0}
            <div>
              <h4 class="text-xs font-semibold text-slate-700 mb-1.5">
                Subscribed Event Topics ({inspectorItem.events.length})
              </h4>
              <div class="flex flex-col gap-1">
                {#each inspectorItem.events as evTopic}
                  <code class="text-[11px] font-code-mono bg-vermilion-glow/10 text-primary px-2.5 py-1 rounded border border-vermilion-glow/20 break-all">
                    {evTopic}
                  </code>
                {/each}
              </div>
            </div>
          {/if}

          {#if inspectorItem.schema}
            <div>
              <h4 class="text-xs font-semibold text-slate-700 mb-1.5">
                JSON Schema
              </h4>
              <pre class="text-[11px] font-code-mono bg-slate-900 text-teal-active p-3 rounded-xl overflow-x-auto max-h-72">{JSON.stringify(inspectorItem.schema, null, 2)}</pre>
            </div>
          {/if}

          {#if inspectorItem.commandRef || inspectorItem.queryRef}
            <button
              type="button"
              onclick={() => openInWorkbenchFromInspector(inspectorItem!)}
              class="w-full py-2.5 px-4 rounded-xl bg-petroleum-void hover:bg-petroleum-surface text-white font-headline-sm text-xs font-semibold flex items-center justify-center gap-2 transition-colors cursor-pointer"
            >
              <span class="material-symbols-outlined text-[16px] text-teal-active">terminal</span>
              <span>
                Open in Live Workbench
              </span>
            </button>
          {/if}
        {:else}
          <div class="flex-1 flex flex-col items-center justify-center text-center py-12 px-4 text-slate-400">
            <span class="material-symbols-outlined text-[32px] mb-2 text-slate-300">touch_app</span>
            <p class="font-headline-sm text-xs font-semibold text-slate-600">
              Contextual Inspector
            </p>
            <p class="text-xs text-slate-400 mt-1">
              Select any Command, Aggregate, Read Model, Query, or Event Consumer card on the canvas to inspect its JSON Schema and jump to the Workbench.
            </p>
          </div>
        {/if}
      </aside>
    </main>
  {/if}
</div>
