/// macOS Apple Events handler for opening files from Finder.
///
/// When a .log file is double-clicked in Finder (or opened with "Open With"),
/// macOS sends a kAEOpenDocuments Apple Event to the app. This module registers
/// a handler via NSAppleEventManager to receive those events and forward the
/// file paths into a channel that the main app can poll.
///
/// Usage (two-phase init required because NSApplication must exist first):
///   1. Call `create_file_receiver()` in main() to set up the channel.
///   2. Call `register_apple_event_handler()` inside LoglineApp::new() after
///      eframe/winit has created NSApplication.

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::{c_char, c_void, CStr};
    use std::path::PathBuf;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::{LazyLock, Mutex};

    // Static sender shared between the Objective-C callback and the Rust app.
    static FILE_SENDER: LazyLock<Mutex<Option<Sender<PathBuf>>>> =
        LazyLock::new(|| Mutex::new(None));

    // Apple Event FourCC constants
    // kCoreEventClass = 'aevt' = 0x61657674
    const K_CORE_EVENT_CLASS: u32 = 0x6165_7674;
    // kAEOpenDocuments = 'odoc' = 0x6f646f63
    const K_AE_OPEN_DOCUMENTS: u32 = 0x6f64_6f63;
    // keyDirectObject = '----' = 0x2d2d2d2d
    const KEY_DIRECT_OBJECT: u32 = 0x2d2d_2d2d;

    // ── Raw libobjc + CoreServices runtime bindings ─────────────────────────
    type ObjcId = *mut c_void;
    type ObjcClass = *mut c_void;
    type ObjcSel = *const c_void;

    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> ObjcClass;
        fn sel_registerName(name: *const c_char) -> ObjcSel;
        fn objc_allocateClassPair(
            superclass: ObjcClass,
            name: *const c_char,
            extra_bytes: usize,
        ) -> ObjcClass;
        fn class_addMethod(
            cls: ObjcClass,
            sel: ObjcSel,
            imp: unsafe extern "C" fn(),
            types: *const c_char,
        ) -> bool;
        fn objc_registerClassPair(cls: ObjcClass);
    }

    // objc_msgSend typed variants – all link to the same symbol; the ABI on
    // arm64 is uniform (no _stret variants needed). The multiple declarations
    // with different signatures are intentional (idiomatic ObjC C-level usage).
    #[allow(clashing_extern_declarations)]
    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        // (recv, sel) -> id
        #[link_name = "objc_msgSend"]
        fn msg_send_id(recv: ObjcId, sel: ObjcSel) -> ObjcId;

        // (recv, sel, u32) -> id  – paramDescriptorForKeyword:
        #[link_name = "objc_msgSend"]
        fn msg_send_id_u32(recv: ObjcId, sel: ObjcSel, kw: u32) -> ObjcId;

        // (recv, sel) -> isize  – numberOfItems
        #[link_name = "objc_msgSend"]
        fn msg_send_isize(recv: ObjcId, sel: ObjcSel) -> isize;

        // (recv, sel, isize) -> id  – descriptorAtIndex:
        #[link_name = "objc_msgSend"]
        fn msg_send_id_isize(recv: ObjcId, sel: ObjcSel, idx: isize) -> ObjcId;

        // (recv, sel) -> *const c_char  – UTF8String
        #[link_name = "objc_msgSend"]
        fn msg_send_cstr(recv: ObjcId, sel: ObjcSel) -> *const c_char;

        // (recv, sel) -> void  – retain
        #[link_name = "objc_msgSend"]
        fn msg_send_void(recv: ObjcId, sel: ObjcSel);

        // setEventHandler:andSelector:forEventClass:andEventID: -> void
        #[link_name = "objc_msgSend"]
        fn msg_send_set_event_handler(
            recv: ObjcId,
            sel: ObjcSel,
            handler: ObjcId,
            handler_sel: ObjcSel,
            event_class: u32,
            event_id: u32,
        );
    }

    // ── Lazy selectors ───────────────────────────────────────────────────────
    fn sel(name: &CStr) -> ObjcSel {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    // ── Apple Event handler (ObjC IMP) ───────────────────────────────────────
    /// Called by the ObjC runtime when macOS delivers kAEOpenDocuments.
    /// Signature: - (void)handleOpenDocuments:(id)event withReplyEvent:(id)reply
    /// Type encoding: "v@:@@"
    unsafe extern "C" fn handle_open_documents(
        _self: ObjcId,
        _cmd: ObjcSel,
        event: ObjcId,
        _reply: ObjcId,
    ) {
        let sel_param = sel(c"paramDescriptorForKeyword:");
        let desc = msg_send_id_u32(event, sel_param, KEY_DIRECT_OBJECT);
        if desc.is_null() {
            return;
        }

        let sel_count = sel(c"numberOfItems");
        let sel_index = sel(c"descriptorAtIndex:");
        let sel_file_url = sel(c"fileURLValue");
        let sel_path = sel(c"path");
        let sel_utf8 = sel(c"UTF8String");

        let count = msg_send_isize(desc, sel_count);
        for i in 1..=count {
            let item = msg_send_id_isize(desc, sel_index, i);
            if item.is_null() {
                continue;
            }
            let url = msg_send_id(item, sel_file_url);
            if url.is_null() {
                continue;
            }
            let path_obj = msg_send_id(url, sel_path);
            if path_obj.is_null() {
                continue;
            }
            let cstr_ptr = msg_send_cstr(path_obj, sel_utf8);
            if cstr_ptr.is_null() {
                continue;
            }
            let path_str = CStr::from_ptr(cstr_ptr).to_string_lossy().into_owned();
            if !path_str.is_empty() {
                tracing::info!("Apple Event: opening file {:?}", path_str);
                if let Ok(guard) = FILE_SENDER.lock() {
                    if let Some(ref sender) = *guard {
                        let _ = sender.send(PathBuf::from(path_str));
                    }
                }
            }
        }
    }

    /// Phase 1: Create the channel and return the Receiver.
    /// Call this from main() before eframe::run_native so the Receiver is ready.
    pub fn create_receiver() -> Receiver<PathBuf> {
        let (tx, rx) = channel::<PathBuf>();
        if let Ok(mut guard) = FILE_SENDER.lock() {
            *guard = Some(tx);
        }
        rx
    }

    /// Phase 2: Register the Apple Event handler with NSAppleEventManager.
    /// Must be called AFTER NSApplication has been created (i.e., inside LoglineApp::new()).
    pub fn register_handler() {
        unsafe {
            // -- 1. Create "LoglineOpenFileHandler" ObjC class ----------------
            let nsobject_cls = objc_getClass(c"NSObject".as_ptr());
            let class_name = c"LoglineOpenFileHandler";

            // allocateClassPair returns NULL if the class already exists.
            let new_cls = objc_allocateClassPair(nsobject_cls, class_name.as_ptr(), 0);
            if !new_cls.is_null() {
                // Method encoding: void, id(self), SEL(_cmd), id(event), id(reply)
                let handler_sel = sel(c"handleOpenDocuments:withReplyEvent:");
                let imp: unsafe extern "C" fn() = std::mem::transmute(
                    handle_open_documents as unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, ObjcId),
                );
                class_addMethod(new_cls, handler_sel, imp, c"v@:@@".as_ptr());
                objc_registerClassPair(new_cls);
            }

            // -- 2. Instantiate the handler object ----------------------------
            let handler_cls = objc_getClass(class_name.as_ptr());
            let handler = msg_send_id(handler_cls, sel(c"new"));
            msg_send_void(handler, sel(c"retain"));

            // -- 3. Register with NSAppleEventManager -------------------------
            let mgr_cls = objc_getClass(c"NSAppleEventManager".as_ptr());
            let mgr = msg_send_id(mgr_cls, sel(c"sharedAppleEventManager"));

            let register_sel = sel(c"setEventHandler:andSelector:forEventClass:andEventID:");
            let handler_method_sel = sel(c"handleOpenDocuments:withReplyEvent:");
            msg_send_set_event_handler(
                mgr,
                register_sel,
                handler,
                handler_method_sel,
                K_CORE_EVENT_CLASS,
                K_AE_OPEN_DOCUMENTS,
            );

            tracing::info!("Registered macOS Apple Events handler for kAEOpenDocuments");
        }
    }
}

/// Phase 1: Create the file-open channel. Call from main() before eframe::run_native.
#[cfg(target_os = "macos")]
pub fn create_file_receiver() -> std::sync::mpsc::Receiver<std::path::PathBuf> {
    imp::create_receiver()
}

/// Phase 2: Register NSAppleEventManager handler. Call from LoglineApp::new() after NSApp exists.
#[cfg(target_os = "macos")]
pub fn register_apple_event_handler() {
    imp::register_handler();
}

// ── non-macOS stubs ──────────────────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
pub fn create_file_receiver() -> std::sync::mpsc::Receiver<std::path::PathBuf> {
    let (_tx, rx) = std::sync::mpsc::channel();
    rx
}

#[cfg(not(target_os = "macos"))]
pub fn register_apple_event_handler() {}
