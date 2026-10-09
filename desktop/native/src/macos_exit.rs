//! Cocoa termination is distinct from Tauri's event-loop exit request.
use objc2::{
    class, msg_send,
    runtime::{AnyObject, Imp, Sel},
    sel,
};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};

struct Hook {
    app: tauri::AppHandle,
    allowed: Arc<AtomicBool>,
}
static HOOK: OnceLock<Hook> = OnceLock::new();

unsafe extern "C-unwind" fn should_terminate(
    _delegate: *mut AnyObject,
    _selector: Sel,
    _sender: *mut AnyObject,
) -> usize {
    let Some(hook) = HOOK.get() else {
        return 0;
    };
    if hook.allowed.load(Ordering::Acquire) {
        return 1;
    }
    if let Some(window) = hook.app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = hook.app.emit("exit-requested", "cocoa-terminate");
    // NSTerminateCancel. The explicit Save/Don't Save/Cancel path calls finish_exit.
    0
}

pub fn install(app: tauri::AppHandle, allowed: Arc<AtomicBool>) -> Result<(), String> {
    HOOK.set(Hook { app, allowed })
        .map_err(|_| "termination guard already installed")?;
    // Tao 0.37.1 has no applicationShouldTerminate: method: Cmd-Q / Dock Quit
    // otherwise bypass RunEvent::ExitRequested and discard dirty drafts.
    // Add only this documented delegate method, retaining all Tao callbacks.
    // Remove this hook when Tao supplies equivalent cancellable Cocoa termination.
    // External evidence: https://github.com/tauri-apps/tauri/issues/12978
    unsafe {
        let application: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![application, delegate];
        let delegate = delegate
            .as_ref()
            .ok_or("Cocoa application delegate missing")?;
        let class = delegate.class();
        let callback: Imp = std::mem::transmute::<
            unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize,
            Imp,
        >(should_terminate);
        // Objective-C ABI: NSUInteger return, object self/sender, selector argument.
        if !objc2::ffi::class_addMethod(
            class as *const _ as *mut _,
            sel!(applicationShouldTerminate:),
            callback,
            c"Q@:@".as_ptr(),
        )
        .as_bool()
        {
            return Err("Cocoa termination guard could not be installed".into());
        }
    }
    Ok(())
}
