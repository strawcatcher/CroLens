export type ClientErrorType =
  | "unhandled_rejection"
  | "render_error"
  | "api_error";

export type ClientErrorEvent = {
  type: ClientErrorType;
  message: string;
  stack?: string;
  traceId?: string;
  url: string;
  timestamp: number;
};

let enabled = false;
let sentryEnabled = false;
let sentryInitPromise: Promise<void> | null = null;
let sentryModule: null | typeof import("@sentry/react") = null;
const buffered: Array<{ event: ClientErrorEvent; original?: unknown }> = [];
const MAX_BUFFERED_EVENTS = 20;

function getUrl() {
  try {
    return typeof window !== "undefined" && window.location
      ? window.location.href
      : "";
  } catch {
    return "";
  }
}

function extractMessage(err: unknown) {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

function extractStack(err: unknown) {
  if (err instanceof Error) return err.stack;
  if (!err || typeof err !== "object") return undefined;
  const stack = (err as { stack?: unknown }).stack;
  return typeof stack === "string" ? stack : undefined;
}

function extractTraceId(err: unknown) {
  if (!err || typeof err !== "object") return undefined;
  const candidate = err as { traceId?: unknown; trace_id?: unknown };
  const traceId =
    typeof candidate.traceId === "string"
      ? candidate.traceId
      : typeof candidate.trace_id === "string"
        ? candidate.trace_id
        : undefined;
  return traceId && traceId.trim().length > 0 ? traceId : undefined;
}

function getSentryDsn() {
  const raw =
    typeof import.meta.env.VITE_SENTRY_DSN === "string"
      ? import.meta.env.VITE_SENTRY_DSN.trim()
      : "";
  return raw.length > 0 ? raw : null;
}

function getSentryEnvironment() {
  const raw =
    typeof import.meta.env.VITE_SENTRY_ENVIRONMENT === "string"
      ? import.meta.env.VITE_SENTRY_ENVIRONMENT.trim()
      : "";
  return raw.length > 0 ? raw : import.meta.env.MODE;
}

function enqueue(event: ClientErrorEvent, original?: unknown) {
  buffered.push({ event, original });
  if (buffered.length <= MAX_BUFFERED_EVENTS) return;
  buffered.splice(0, buffered.length - MAX_BUFFERED_EVENTS);
}

async function initSentryIfConfigured() {
  if (sentryInitPromise) return sentryInitPromise;

  const dsn = getSentryDsn();
  if (!dsn) return;

  sentryInitPromise = (async () => {
    try {
      sentryModule = await import("@sentry/react");
      sentryModule.init({
        dsn,
        environment: getSentryEnvironment(),
        enabled: true,
        defaultIntegrations: false,
        integrations: [],
      });
      sentryEnabled = true;

      while (buffered.length > 0) {
        const item = buffered.shift();
        if (!item) break;
        captureToSentry(item.event, item.original);
      }
    } catch (err) {
      sentryEnabled = false;
      sentryModule = null;
      console.warn("[monitoring] Failed to init Sentry:", err);
    } finally {
      sentryInitPromise = null;
    }
  })();

  return sentryInitPromise;
}

function captureToSentry(payload: ClientErrorEvent, original?: unknown) {
  if (!sentryEnabled || !sentryModule) return;
  const sentry = sentryModule;

  sentry.withScope((scope) => {
    scope.setTag("type", payload.type);
    if (payload.traceId) scope.setTag("traceId", payload.traceId);
    scope.setExtra("url", payload.url);
    scope.setExtra("timestamp", payload.timestamp);
    if (payload.stack) scope.setExtra("stack", payload.stack);

    scope.setLevel(payload.type === "api_error" ? "warning" : "error");

    if (original instanceof Error) {
      sentry.captureException(original);
      return;
    }

    sentry.captureMessage(payload.message);
  });
}

export function reportClientError(payload: ClientErrorEvent, original?: unknown) {
  if (!enabled) {
    if (getSentryDsn()) {
      enqueue(payload, original);
      void initSentryIfConfigured();
    }
    return;
  }

  if (sentryEnabled) {
    captureToSentry(payload, original);
  } else if (getSentryDsn()) {
    enqueue(payload, original);
    void initSentryIfConfigured();
  }

  if (payload.type === "api_error") {
    console.warn("[monitoring]", payload);
    return;
  }
  console.error("[monitoring]", payload);
}

export function reportUnhandledRejection(reason: unknown) {
  const payload: ClientErrorEvent = {
    type: "unhandled_rejection",
    message: extractMessage(reason),
    stack: extractStack(reason),
    traceId: extractTraceId(reason),
    url: getUrl(),
    timestamp: Date.now(),
  };

  reportClientError(payload, reason);
}

export function reportRenderError(error: Error, componentStack?: string | null) {
  const extra = componentStack?.trim();
  const stack = [error.stack, extra].filter(Boolean).join("\n");

  const payload: ClientErrorEvent = {
    type: "render_error",
    message: extractMessage(error),
    stack: stack.length > 0 ? stack : undefined,
    url: getUrl(),
    timestamp: Date.now(),
  };

  reportClientError(payload, error);
}

export function reportApiError(error: unknown) {
  const payload: ClientErrorEvent = {
    type: "api_error",
    message: extractMessage(error),
    stack: extractStack(error),
    traceId: extractTraceId(error),
    url: getUrl(),
    timestamp: Date.now(),
  };

  reportClientError(payload, error);
}

export function setupMonitoring() {
  if (enabled) return;
  enabled = true;

  if (getSentryDsn()) {
    void initSentryIfConfigured();
  }

  if (typeof window === "undefined") return;

  window.addEventListener("unhandledrejection", (event) => {
    reportUnhandledRejection(event.reason);
  });
}
