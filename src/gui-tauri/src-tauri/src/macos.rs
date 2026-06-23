// FlowDesk macOS-specific helpers.
//
// Mirrors the legacy main.cpp accessibility check. See docs/design/tauri-gui.md §3.6.
//
// On macOS, ApplicationServices (AXIsProcessTrustedWithOptions) is linked
// automatically by the system frameworks; we declare the extern directly.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

#[cfg(target_os = "macos")]
mod inner {
    use std::ffi::c_void;
    use std::os::raw::{c_char, c_int};

    // AXIsProcessTrustedWithOptions lives in ApplicationServices, which the
    // Tauri macOS bundle links by default (we declared the framework in
    // tauri.conf.json). extern "C" is enough; no link attribute needed on macOS.
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *mut c_void) -> c_int;
    }

    const AX_TRUSTED_CHECK_OPTION_PROMPT: &str = "AXTrustedCheckOptionPrompt";

    /// Returns true if the current process has Accessibility permission.
    /// If `prompt` is true, macOS opens System Settings → Privacy →
    /// Accessibility the first time (matches the legacy GUI behaviour).
    pub fn is_trusted(prompt: bool) -> bool {
        if !prompt {
            // No-prompt query: pass NULL options.
            return unsafe { AXIsProcessTrustedWithOptions(std::ptr::null_mut()) != 0 };
        }

        // Build a minimal CFDictionary { AXTrustedCheckOptionPrompt: true }
        // via the C CoreFoundation API. We avoid CFBooleanCreate (its symbol
        // is fragile across SDK versions) by reusing kCFBooleanTrue, which is
        // a global constant exported by CoreFoundation.
        unsafe {
            let key = cf_string_create(AX_TRUSTED_CHECK_OPTION_PROMPT);
            if key.is_null() {
                // Fall back to no prompt if we couldn't create the key.
                return AXIsProcessTrustedWithOptions(std::ptr::null_mut()) != 0;
            }
            let val: *const c_void = kCFBooleanTrue;

            let keys = [key as *const c_void];
            let vals = [val as *const c_void];
            let opts = cf_dictionary_create(&keys, &vals);

            let trusted = AXIsProcessTrustedWithOptions(opts as *mut c_void);

            cf_release(opts as *const c_void);
            cf_release(key as *const c_void);
            // kCFBooleanTrue is a global constant — do NOT release it.

            trusted != 0
        }
    }

    // Minimal CoreFoundation C bindings (kept private to this module).
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: *mut c_void,
            cstr: *const c_char,
            encoding: u32,
        ) -> *mut c_void;
        fn CFDictionaryCreate(
            alloc: *mut c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *mut c_void;
        fn CFRelease(cf: *const c_void);
        // kCFBooleanTrue is a global constant of type CFBooleanRef exported
        // by CoreFoundation. We access it as an extern static.
        static kCFBooleanTrue: *const c_void;
    }

    // kCFStringEncodingUTF8
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

    unsafe fn cf_string_create(s: &str) -> *mut c_void {
        let c = std::ffi::CString::new(s).unwrap();
        CFStringCreateWithCString(std::ptr::null_mut(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8)
    }

    unsafe fn cf_dictionary_create(keys: &[*const c_void], vals: &[*const c_void]) -> *mut c_void {
        CFDictionaryCreate(
            std::ptr::null_mut(),
            keys.as_ptr(),
            vals.as_ptr(),
            keys.len() as isize,
            std::ptr::null(),
            std::ptr::null(),
        )
    }

    unsafe fn cf_release(p: *const c_void) {
        if !p.is_null() {
            CFRelease(p);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod inner {
    pub fn is_trusted(_prompt: bool) -> bool {
        true // No accessibility gate on this platform.
    }
}

pub fn is_accessible() -> bool {
    inner::is_trusted(false)
}

/// Trigger the macOS permission prompt (opens System Settings). Returns
/// whether the process is currently trusted; the frontend should re-check
/// periodically because the value flips without an app restart once granted.
pub fn prompt_for_access() -> bool {
    inner::is_trusted(true)
}

// --- Screen Capture permission (macOS 10.15+) ---
// On macOS 15+ (Sequoia) and later, reading the global mouse position via
// CGEventGetLocation requires Screen Recording permission in addition to
// Accessibility. Without it, barriers can capture keyboard + clicks but NOT
// mouse motion — which reads as "slow / drops frames" to the user.

#[cfg(target_os = "macos")]
mod screen_capture {
    use std::os::raw::c_int;

    extern "C" {
        // CoreGraphics (ApplicationServices). Returns true if the calling
        // process already has Screen Recording permission (10.15+).
        fn CGPreflightScreenCaptureAccess() -> c_int;
        // Triggers the system consent prompt if not yet granted (10.15+).
        fn CGRequestScreenCaptureAccess() -> c_int;
    }

    /// Does the process already have Screen Recording permission?
    pub fn preflight() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() != 0 }
    }

    /// Request Screen Recording permission (may open System Settings).
    /// Returns whether permission is granted *now* (may be false until the
    /// user grants it and the app is restarted).
    pub fn request() -> bool {
        unsafe { CGRequestScreenCaptureAccess() != 0 }
    }
}

#[cfg(not(target_os = "macos"))]
mod screen_capture {
    pub fn preflight() -> bool { true }
    pub fn request() -> bool { true }
}

/// Does the barriers subprocess (which inherits our bundle's permissions)
/// have Screen Recording permission? On macOS 15+ this is required for
/// mouse-motion capture alongside Accessibility.
pub fn has_screen_capture() -> bool {
    screen_capture::preflight()
}

/// Request Screen Recording permission (opens System Settings if needed).
pub fn request_screen_capture() -> bool {
    screen_capture::request()
}

/// Open System Settings directly to the Privacy & Security → Accessibility
/// pane. Uses `open` on the special URL scheme; more reliable than the JS
/// opener plugin for macOS-specific schemes.
#[cfg(target_os = "macos")]
pub fn open_accessibility_settings() -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .map(|_| ())
}

#[cfg(not(target_os = "macos"))]
pub fn open_accessibility_settings() -> std::io::Result<()> {
    Ok(())
}
