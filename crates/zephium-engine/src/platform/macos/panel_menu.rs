//! "Open Link in Web Panel" and "Search in Web Panel" in a page's own menu.
//! WebKit hands the browser a link only through its new-window request, so
//! the link item runs WebKit's "Open Link in New Window" and marks that
//! request for a panel. The search item reads the selection itself, from a
//! script world the page cannot reach.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{define_class, sel, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSUserInterfaceItemIdentification};
use objc2_foundation::{NSError, NSString};
use objc2_web_kit::{WKContentWorld, WKWebView};

/// WebKit's own item for a link, whose action the link item reuses.
const OPEN_LINK_IN_NEW_WINDOW: &str = "WKMenuItemIdentifierOpenLinkInNewWindow";
/// WebKit's own item for selected text, after which the search item stands.
const SEARCH_WEB: &str = "WKMenuItemIdentifierSearchWeb";
/// How long a chosen link item waits for WebKit's new-window request.
const INTENT: Duration = Duration::from_secs(3);
/// The selection as the browser reads it, never as the page reports it.
const SELECTION: &str = "String(getSelection() ?? '').trim().slice(0, 1000)";

/// Set when the link item is chosen and taken by the next new-window request.
#[derive(Clone, Default)]
pub(crate) struct PanelIntent(Rc<Cell<Option<Instant>>>);

impl PanelIntent {
    /// Whether the request now arriving was asked for by the item.
    pub(crate) fn take(&self) -> bool {
        self.0.take().is_some_and(|at| at.elapsed() < INTENT)
    }
}

/// What the search item does with the text it reads.
pub(crate) type SearchSink = Rc<dyn Fn(String)>;

pub(crate) enum TargetIvars {
    Link {
        intent: PanelIntent,
        original: Retained<NSMenuItem>,
    },
    Search {
        view: Weak<WKWebView>,
        search: SearchSink,
    },
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
            let TargetIvars::Link { intent, original } = self.ivars() else {
                return;
            };
            let (action, target) = (original.action(), original.target());
            let Some(action) = action else {
                return;
            };
            intent.0.set(Some(Instant::now()));
            let app = NSApplication::sharedApplication(self.mtm());
            // SAFETY: WebKit's own item, sent its own action from itself, as
            // choosing it would.
            let sent = unsafe { app.sendAction_to_from(action, target.as_deref(), Some(original)) };
            if !sent {
                intent.0.set(None);
            }
        }

        #[unsafe(method(searchInPanel:))]
        fn search_in_panel(&self, _sender: Option<&AnyObject>) {
            let TargetIvars::Search { view, search } = self.ivars() else {
                return;
            };
            let Some(view) = view.load() else {
                return;
            };
            let search = search.clone();
            let done = RcBlock::new(move |result: *mut AnyObject, _error: *mut NSError| {
                // SAFETY: WebKit lends the script's result for this call.
                let Some(result) = (unsafe { result.as_ref() }) else {
                    return;
                };
                if let Some(text) = result.downcast_ref::<NSString>() {
                    let text = text.to_string();
                    if !text.is_empty() {
                        search(text);
                    }
                }
            });
            let world = unsafe { WKContentWorld::defaultClientWorld(self.mtm()) };
            // SAFETY: a fixed read-only script in the browser's own world.
            unsafe {
                view.evaluateJavaScript_inFrame_inContentWorld_completionHandler(
                    &NSString::from_str(SELECTION),
                    None,
                    &world,
                    Some(&done),
                );
            }
        }
    }
);

/// The browser's items held for the menu now open. A menu item does not
/// retain its target, so they live here until the next menu.
#[derive(Default)]
pub(crate) struct KeptTargets(RefCell<Vec<Retained<PanelMenuTarget>>>);

fn find(menu: &NSMenu, identifier: &str) -> Option<(usize, Retained<NSMenuItem>)> {
    let wanted = NSString::from_str(identifier);
    menu.itemArray()
        .iter()
        .enumerate()
        .find_map(|(index, item)| {
            item.identifier()
                .is_some_and(|found| found.isEqualToString(&wanted))
                .then_some((index, item))
        })
}

fn insert(
    menu: &NSMenu,
    after: usize,
    title: &str,
    action: objc2::runtime::Sel,
    target: Retained<PanelMenuTarget>,
    mtm: MainThreadMarker,
    kept: &KeptTargets,
) {
    // SAFETY: a new item with a selector the target implements.
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(""),
        )
    };
    // SAFETY: the target is kept alive in `kept` while the menu is shown.
    unsafe { item.setTarget(Some(&target)) };
    menu.insertItem_atIndex(&item, isize::try_from(after + 1).unwrap_or(isize::MAX));
    kept.0.borrow_mut().push(target);
}

fn target(ivars: TargetIvars, mtm: MainThreadMarker) -> Retained<PanelMenuTarget> {
    let target = PanelMenuTarget::alloc(mtm).set_ivars(ivars);
    // SAFETY: plain NSObject init on the main thread.
    unsafe { objc2::msg_send![super(target), init] }
}

/// Adds the panel items after WebKit's own link and search items, when the
/// menu has them.
pub(crate) fn add_panel_items(
    menu: &NSMenu,
    view: &WKWebView,
    intent: &PanelIntent,
    search: &SearchSink,
    kept: &KeptTargets,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    kept.0.borrow_mut().clear();
    let labels = crate::page_menu::labels();
    if let Some((index, original)) = find(menu, OPEN_LINK_IN_NEW_WINDOW) {
        let link = target(
            TargetIvars::Link {
                intent: intent.clone(),
                original,
            },
            mtm,
        );
        insert(
            menu,
            index,
            &labels.open_link_in_panel,
            sel!(openInPanel:),
            link,
            mtm,
            kept,
        );
    }
    if let Some((index, _)) = find(menu, SEARCH_WEB) {
        let selection = target(
            TargetIvars::Search {
                view: Weak::from_retained(&view.retain()),
                search: search.clone(),
            },
            mtm,
        );
        insert(
            menu,
            index,
            &labels.search_in_panel,
            sel!(searchInPanel:),
            selection,
            mtm,
            kept,
        );
    }
}
