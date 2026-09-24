use std::sync::OnceLock;

use objc2::{
    MainThreadMarker,
    runtime::{AnyObject, Sel},
    sel,
};
use objc2_app_kit::{NSApplication, NSApplicationTerminateReply};
use tauri::{AppHandle, Manager};

static APP: OnceLock<AppHandle> = OnceLock::new();

extern "C-unwind" fn should_terminate(
    _delegate: &AnyObject,
    _selector: Sel,
    _application: &NSApplication,
) -> NSApplicationTerminateReply {
    if let Some(window) = APP.get().and_then(|app| app.get_webview_window("main")) {
        if let Err(error) = window.close() {
            eprintln!("Could not request guarded macOS quit: {error}");
        }
        NSApplicationTerminateReply::TerminateCancel
    } else {
        NSApplicationTerminateReply::TerminateNow
    }
}

pub fn install_quit_guard(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mtm = MainThreadMarker::new().ok_or("Quit guard must be installed on the main thread")?;
    let delegate = NSApplication::sharedApplication(mtm)
        .delegate()
        .ok_or("Missing macOS application delegate")?;
    // SAFETY: every Objective-C protocol object is an AnyObject.
    let delegate = unsafe { objc2::rc::Retained::cast_unchecked::<AnyObject>(delegate) };
    if delegate
        .class()
        .instance_method(sel!(applicationShouldTerminate:))
        .is_some()
    {
        return Err(
            "Application delegate already owns termination; review quit guard integration".into(),
        );
    }
    APP.set(app.clone())
        .map_err(|_| "Quit guard already installed")?;

    // Tao 0.35 does not implement applicationShouldTerminate:, and muda's
    // predefined Quit calls AppKit terminate: directly, bypassing ExitRequested.
    // Add the optional delegate callback without replacing Tao's delegate or any
    // existing method. This also protects Dock Quit (GUI-SHELL-LIFECYCLE-001).
    // Revisit when Tao routes native termination through a cancellable event.
    // SAFETY: AppKit invokes this on the main thread. The callback matches the
    // NSUInteger return and object arguments of applicationShouldTerminate: on
    // supported 64-bit macOS. The runtime owns the class; no layout is changed.
    let added = unsafe {
        objc2::ffi::class_addMethod(
            delegate.class() as *const _ as *mut _,
            sel!(applicationShouldTerminate:),
            std::mem::transmute::<
                extern "C-unwind" fn(
                    &AnyObject,
                    Sel,
                    &NSApplication,
                ) -> NSApplicationTerminateReply,
                unsafe extern "C-unwind" fn(),
            >(should_terminate),
            c"Q@:@".as_ptr(),
        )
    };
    if !added.as_bool() {
        return Err(
            "Could not install macOS quit guard without replacing an existing method".into(),
        );
    }
    Ok(())
}
