//! "Open Link in Web Panel" in a page's own menu. WebKit hands the link to
//! the browser only through its new-window request, so the item runs
//! WebKit's "Open Link in New Window" and marks that request for a panel.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{define_class, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSUserInterfaceItemIdentification};
use objc2_foundation::NSString;

/// WebKit's own item for a link, whose action this item reuses.
const OPEN_LINK_IN_NEW_WINDOW: &str = "WKMenuItemIdentifierOpenLinkInNewWindow";
/// How long a chosen item waits for WebKit's new-window request.
const INTENT: Duration = Duration::from_secs(3);

/// Set when the item is chosen and taken by the next new-window request.
#[derive(Clone, Default)]
pub(crate) struct PanelIntent(Rc<Cell<Option<Instant>>>);

impl PanelIntent {
    /// Whether the request now arriving was asked for by the item.
    pub(crate) fn take(&self) -> bool {
        self.0.take().is_some_and(|at| at.elapsed() < INTENT)
    }
}

pub(crate) struct TargetIvars {
    intent: PanelIntent,
    original: Retained<NSMenuItem>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "ZephiumPanelMenuTarget"]
    #[ivars = TargetIvars]
    pub(crate) struct PanelMenuTarget;

    unsafe impl NSObjectProtocol for PanelMenuTarget {}

    impl PanelMenuTarget {
        #[unsafe(method(openInPanel:))]
        fn open_in_panel(&self, _sender: Option<&AnyObject>) {
            let ivars = self.ivars();
            let (action, target) = (ivars.original.action(), ivars.original.target());
            let Some(action) = action else {
                return;
            };
            ivars.intent.0.set(Some(Instant::now()));
            let app = NSApplication::sharedApplication(self.mtm());
            // SAFETY: WebKit's own item, sent its own action from itself, as
            // choosing it would.
            let sent = unsafe { app.sendAction_to_from(action, target.as_deref(), Some(&ivars.original)) };
            if !sent {
                ivars.intent.0.set(None);
            }
        }
    }
);

/// Adds the item after WebKit's "Open Link in New Window" when the menu has
/// one. A menu item does not retain its target, so `kept` holds it until
/// the next menu.
pub(crate) fn add_open_in_panel(
    menu: &NSMenu,
    intent: &PanelIntent,
    kept: &RefCell<Option<Retained<PanelMenuTarget>>>,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let wanted = NSString::from_str(OPEN_LINK_IN_NEW_WINDOW);
    let items = menu.itemArray();
    let Some((index, original)) = items.iter().enumerate().find(|(_, item)| {
        item.identifier()
            .is_some_and(|identifier| identifier.isEqualToString(&wanted))
    }) else {
        return;
    };
    let target = PanelMenuTarget::alloc(mtm).set_ivars(TargetIvars {
        intent: intent.clone(),
        original: original.clone(),
    });
    // SAFETY: plain NSObject init on the main thread.
    let target: Retained<PanelMenuTarget> = unsafe { objc2::msg_send![super(target), init] };
    // SAFETY: a new item with a selector the target implements.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str("Open Link in Web Panel"),
            Some(sel!(openInPanel:)),
            &NSString::from_str(""),
        )
    };
    // SAFETY: the target is kept alive in `kept` while the menu is shown.
    unsafe { item.setTarget(Some(&target)) };
    let at = isize::try_from(index + 1).unwrap_or(isize::MAX);
    menu.insertItem_atIndex(&item, at);
    kept.replace(Some(target));
}
