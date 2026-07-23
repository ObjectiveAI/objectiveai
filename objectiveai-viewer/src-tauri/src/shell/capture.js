// Injected into EVERY viewer webview (chrome and content alike) as an
// initialization script — it runs at document start, BEFORE any page
// script, so it captures webviews whose own bundle never boots. Wraps
// every console method plus uncaught errors and unhandled rejections
// and forwards them to the Rust log store (`logs_report`), buffering
// until the Tauri IPC bootstrap is reachable. Zero cooperation needed
// from the page.
(function () {
  if (window.__OBJECTIVEAI_CAPTURE__) return;
  window.__OBJECTIVEAI_CAPTURE__ = true;

  var pending = [];
  var draining = false;

  function invoke(entry) {
    var internals = window.__TAURI_INTERNALS__;
    if (!internals || typeof internals.invoke !== "function") return false;
    try {
      var result = internals.invoke("logs_report", entry);
      // A rejected report must NOT surface as an unhandled rejection —
      // the rejection handler would re-report it, forever.
      if (result && typeof result.catch === "function") {
        result.catch(function () {});
      }
      return true;
    } catch (e) {
      return false;
    }
  }

  function drain() {
    if (draining) return;
    draining = true;
    var timer = setInterval(function () {
      while (pending.length > 0) {
        if (!invoke(pending[0])) return; // IPC not up yet — retry later
        pending.shift();
      }
      clearInterval(timer);
      draining = false;
    }, 50);
  }

  function send(level, message, detail) {
    var entry = {
      level: level,
      message: message,
      detail: detail == null ? null : detail,
    };
    if (pending.length > 0 || !invoke(entry)) {
      // Keep ORDER: once anything is queued, everything queues.
      if (pending.length < 500) pending.push(entry);
      drain();
    }
  }

  function fmt(value) {
    try {
      if (typeof value === "string") return value;
      if (value instanceof Error) return value.name + ": " + value.message;
      var json = JSON.stringify(value);
      return json === undefined ? String(value) : json;
    } catch (e) {
      try {
        return String(value);
      } catch (e2) {
        return "<unprintable>";
      }
    }
  }

  function fmtAll(args) {
    var parts = [];
    for (var i = 0; i < args.length; i++) parts.push(fmt(args[i]));
    var joined = parts.join(" ");
    return joined.length > 4096 ? joined.slice(0, 4096) + "…" : joined;
  }

  var levels = ["log", "info", "warn", "error", "debug", "trace"];
  for (var i = 0; i < levels.length; i++) {
    (function (level) {
      var original = console[level];
      console[level] = function () {
        send(level, fmtAll(arguments), null);
        if (typeof original === "function") {
          return original.apply(console, arguments);
        }
      };
    })(levels[i]);
  }

  window.addEventListener("error", function (event) {
    var detail =
      event.error && typeof event.error.stack === "string"
        ? event.error.stack
        : null;
    var message = event.message ? String(event.message) : fmt(event.error);
    send("uncaught", message, detail);
  });

  window.addEventListener("unhandledrejection", function (event) {
    var reason = event.reason;
    var detail =
      reason && typeof reason.stack === "string" ? reason.stack : null;
    send("unhandledrejection", fmt(reason), detail);
  });
})();
