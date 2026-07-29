//! The script→viewer IPC bridge, renderer-process side.
//!
//! An injected browser-tab script has no Tauri IPC, no `plugin://`
//! origin, and no SDK — it is a classic script in a FOREIGN page. This
//! module gives it one narrow, token-guarded lane back to the viewer:
//!
//! 1. [`RendererBridge`] registers a native function
//!    ([`BRIDGE_FN`], `__objectiveai_ipc`) on every browser-tab main
//!    frame's `window` at context creation — BEFORE any page script
//!    runs, pinned `READONLY | DONTDELETE | DONTENUM` so the page can
//!    never replace it with a token-stealing shim.
//! 2. The injected prelude (built in `shell::browser`, where the
//!    per-spawn token is minted) captures that function and the token
//!    in a closure and exposes the public surface:
//!    `window.__objectiveai.send(payload) -> boolean`.
//! 3. [`BridgeFn`] forwards `(token, json)` to the BROWSER process as
//!    the [`BRIDGE_MESSAGE`] process message; `ViewerClient` over in
//!    `runtime.rs` token-checks it and routes the parsed payload into
//!    the spawning tab's mailbox (`TabMail`), where the plugin tab
//!    drains it with its ordinary `subscribeViewerTab(key)`.
//!
//! The page shares the JS world with the injected script (CEF has no
//! isolated worlds for `execute_java_script`), so the page CAN call
//! the genuine function — but without the closure-held token its
//! messages are dropped browser-side, and it cannot observe the
//! wrapper's call arguments. Delivery is fire-and-forget: the boolean
//! only says "handed to CEF", not "delivered".
//!
//! This code runs in the RENDERER subprocess (`run_helper_and_exit`
//! passes the same `ViewerApp`), so it must not touch tauri or any
//! viewer state — it is wired purely through CEF.

use cef::rc::Rc as _;
use cef::*;

/// The process-message name the bridge rides.
pub(crate) const BRIDGE_MESSAGE: &str = "objectiveai:bridge";
/// The native function name registered on `window`. The public surface
/// is the prelude's `window.__objectiveai.send`; this raw name is an
/// implementation detail scripts should not call directly.
pub(crate) const BRIDGE_FN: &str = "__objectiveai_ipc";
/// Serialized-payload cap, enforced on BOTH sides of the process
/// boundary (the prelude also refuses client-side): process messages
/// are copied cross-process, and an unbounded payload is a foreign
/// page's lever on the viewer's memory.
pub(crate) const BRIDGE_PAYLOAD_CAP: usize = 256 * 1024;

wrap_v8_handler! {
    struct BridgeFn {}

    impl V8Handler {
        fn execute(
            &self,
            name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            retval: Option<&mut Option<V8Value>>,
            _exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
            let delivered = deliver(name, arguments);
            if let Some(retval) = retval {
                *retval = v8_value_create_bool(delivered as ::std::os::raw::c_int);
            }
            // 1 = handled — the page never sees a V8 exception from
            // us, only the boolean.
            1
        }
    }
}

/// Validate one call and forward it as a process message. `false` for
/// anything malformed — wrong name, arity, non-string args, oversized
/// payload, or a frame CEF cannot resolve.
fn deliver(name: Option<&CefString>, arguments: Option<&[Option<V8Value>]>) -> bool {
    if name.map(|n| n.to_string()).as_deref() != Some(BRIDGE_FN) {
        return false;
    }
    let Some([Some(token), Some(json)]) = arguments else {
        return false;
    };
    if token.is_string() != 1 || json.is_string() != 1 {
        return false;
    }
    let token = CefString::from(&token.string_value()).to_string();
    let json = CefString::from(&json.string_value()).to_string();
    if json.len() > BRIDGE_PAYLOAD_CAP {
        return false;
    }
    // The frame whose context invoked us — the routing key the browser
    // process resolves back to a tab.
    let Some(context) = v8_context_get_current_context() else {
        return false;
    };
    let Some(frame) = context.frame() else {
        return false;
    };
    let message_name = CefString::from(BRIDGE_MESSAGE);
    let Some(mut message) = process_message_create(Some(&message_name)) else {
        return false;
    };
    let Some(args) = message.argument_list() else {
        return false;
    };
    args.set_size(2);
    args.set_string(0, Some(&CefString::from(token.as_str())));
    args.set_string(1, Some(&CefString::from(json.as_str())));
    frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
    true
}

wrap_render_process_handler! {
    pub struct RendererBridge {}

    impl RenderProcessHandler {
        /// Pin the bridge function onto every main-frame `window`
        /// before the page's own scripts run. Registered
        /// unconditionally — the renderer cannot know whether a script
        /// was injected, and a token-less registration is inert (no
        /// token ever validates browser-side).
        fn on_context_created(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            let Some(frame) = frame else { return };
            if frame.is_main() != 1 {
                return;
            }
            let Some(context) = context else { return };
            // The context should already be current inside this
            // callback; entering explicitly is cheap insurance —
            // creating V8 values outside a current context fails.
            let entered = context.enter() == 1;
            let name = CefString::from(BRIDGE_FN);
            let mut handler = BridgeFn::new();
            if let (Some(mut function), Some(global)) = (
                v8_value_create_function(Some(&name), Some(&mut handler)),
                context.global(),
            ) {
                use cef::sys::cef_v8_propertyattribute_t as attr;
                let pinned = V8Propertyattribute::from(
                    attr::V8_PROPERTY_ATTRIBUTE_READONLY
                        | attr::V8_PROPERTY_ATTRIBUTE_DONTDELETE
                        | attr::V8_PROPERTY_ATTRIBUTE_DONTENUM,
                );
                let _ = global.set_value_bykey(Some(&name), Some(&mut function), pinned);
            }
            if entered {
                let _ = context.exit();
            }
        }
    }
}
