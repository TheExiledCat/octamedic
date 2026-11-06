use std::sync::atomic::{AtomicBool, Ordering};

use objc2::{
    class, ffi, msg_send,
    runtime::{AnyClass, AnyObject, Imp, Sel},
    sel,
};

// NSApplicationTerminateReply
const TERMINATE_CANCEL: usize = 0;
const TERMINATE_NOW: usize = 1;

// the app delegate can't reach App, so App keeps these up to date every frame
static UNSAVED: AtomicBool = AtomicBool::new(false);
static QUIT_ASKED: AtomicBool = AtomicBool::new(false);

// applicationShouldTerminate:, given the delegate, the selector and the NSApplication
type ShouldTerminate = unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize;

/// Adds applicationShouldTerminate: to winit's app delegate: quit now if saved, else cancel and
/// let App ask. Call after the event loop exists.
pub fn ask_before_quitting() {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        let Some(delegate) = delegate.as_ref() else {
            return;
        };
        let class = delegate.class() as *const AnyClass as *mut AnyClass;
        // ShouldTerminate's types: an NSUInteger, an object, a selector and an object
        let types = c"Q@:@";
        let imp = std::mem::transmute::<ShouldTerminate, Imp>(should_terminate);
        ffi::class_addMethod(
            class,
            sel!(applicationShouldTerminate:),
            imp,
            types.as_ptr(),
        );
    }
}

/// Every frame, so the delegate knows whether to quit without asking.
pub fn set_unsaved(unsaved: bool) {
    UNSAVED.store(unsaved, Ordering::Relaxed);
}

/// Whether a quit was held back for unsaved changes since the last call.
pub fn take_quit_request() -> bool {
    return QUIT_ASKED.swap(false, Ordering::Relaxed);
}

unsafe extern "C-unwind" fn should_terminate(
    _delegate: *mut AnyObject,
    _selector: Sel,
    _app: *mut AnyObject,
) -> usize {
    return terminate_reply();
}

fn terminate_reply() -> usize {
    if !UNSAVED.load(Ordering::Relaxed) {
        return TERMINATE_NOW;
    }
    QUIT_ASKED.store(true, Ordering::Relaxed);
    return TERMINATE_CANCEL;
}

#[cfg(test)]
mod tests {
    use super::*;

    // one test, since the flags are shared
    #[test]
    fn quits_now_only_when_saved() {
        set_unsaved(false);
        assert_eq!(terminate_reply(), TERMINATE_NOW);
        assert!(!take_quit_request());

        set_unsaved(true);
        assert_eq!(terminate_reply(), TERMINATE_CANCEL);
        assert!(take_quit_request());
        assert!(!take_quit_request());
        set_unsaved(false);
    }
}
