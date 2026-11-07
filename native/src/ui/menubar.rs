//! Native menu bar: File / Edit / View / Insert / Format / Tools / Help.
//!
//! Mirrors the menu structure in src/EditorWorkspace.tsx (lines ~351-424) and
//! the library preset entries for Insert and Format (CHROME_MENUS in
//! @docx-editor.dev/core). Commands follow native/BRIDGE.md:
//!
//! - Session/UI commands -> `UiCommand::Command{name, args}`
//!   Through The Rust Workspace Command Dispatcher.
//! - Formatting commands -> `UiCommand::Exec{name:"editorExec", args:<cmd>}`
//!   with the same payloads as `runFormattingCommand` / the library
//!   `commandForSlot` map (verified against the installed package).
//!
//! # Best-effort items
//!
//! A few preset slots open React dialogs inside the web surface (image file
//! picker, paragraph dialog, format painter, watermark, bookmark dialogs,
//! link). They are sent as `editorExec` guesses; the surface will refuse with
//! a notice if unsupported. Verified payloads come from the library's
//! `commandForSlot` table; dialog-only slots are marked in `menu_entries`.
//!
//! # Recent documents
//!
//! File > Open Recent emits [`UiCommand::FetchRecent`] when opened (throttled
//! to once per 30s). Results are supplied by the integrator via
//! [`crate::ui::store_recents`].

use std::time::{Duration, Instant};

use egui::{vec2, Align2, Color32, FontId, Frame, Id, Margin, Pos2, Rect, Sense, Stroke};
use serde_json::Value;

use crate::state::{AppState, Modal, ViewMode};
use crate::theme::UiColors;

use super::{clear_all_submenus, colors, recents, ChromeAssets, RecentEntry, UiCommand, UiOutput};

const MENU_NAMES: [&str; 7] = ["file", "edit", "view", "insert", "format", "tools", "help"];
const MENU_LABELS: [&str; 7] = ["File", "Edit", "View", "Insert", "Format", "Tools", "Help"];
const MENU_RECTS: &str = "insyn_menu_trigger_rects";
const MENU_POPUP_RECTS: &str = "insyn_menu_popup_rects";
const RECENT_FETCHED: &str = "insyn_recent_fetch_instant";

// ---------------------------------------------------------------------------
// Menu model
// ---------------------------------------------------------------------------

type BoolFn = fn(&AppState) -> bool;

#[derive(Clone, Copy)]
pub enum MenuAction {
    /// `Command{name, args:null}`
    Command(&'static str),
    /// `Command{name, args: <json literal>}`
    CommandJson(&'static str, &'static str),
    /// `Exec{name:"editorExec", args:<json literal>}`
    EditorExec(&'static str),
    /// Top-level `Exec{name, args:<json literal>}`
    Exec(&'static str, &'static str),
    /// Open a native modal.
    Modal(Modal),
    /// Toggle preferences.show_outline + SavePreferences.
    ToggleOutline,
    /// Toggle preferences.show_ruler + SavePreferences.
    ToggleRuler,
    /// Toggle focus mode via Command focusMode {enabled: !current}.
    ToggleFocus,
}

#[derive(Clone, Copy)]
pub enum MenuEntryKind {
    Item {
        label: &'static str,
        shortcut: Option<&'static str>,
        action: MenuAction,
        active: Option<BoolFn>,
        disabled: Option<BoolFn>,
    },
    Separator,
    /// One-level submenu (Open Recent handled separately via RecentSubmenu).
    Submenu {
        id: &'static str,
        label: &'static str,
        items: &'static [MenuEntry],
    },
    /// File > Open Recent (dynamic content).
    RecentSubmenu,
    /// Insert > Table grid picker (6 rows x 6 cols -> insertTable).
    TableGrid,
}

#[derive(Clone, Copy)]
pub struct MenuEntry {
    pub kind: MenuEntryKind,
}

const fn item(
    label: &'static str,
    shortcut: Option<&'static str>,
    action: MenuAction,
) -> MenuEntry {
    MenuEntry {
        kind: MenuEntryKind::Item {
            label,
            shortcut,
            action,
            active: None,
            disabled: None,
        },
    }
}

const fn item_active(
    label: &'static str,
    shortcut: Option<&'static str>,
    action: MenuAction,
    active: BoolFn,
) -> MenuEntry {
    MenuEntry {
        kind: MenuEntryKind::Item {
            label,
            shortcut,
            action,
            active: Some(active),
            disabled: None,
        },
    }
}

const fn item_disabled(
    label: &'static str,
    shortcut: Option<&'static str>,
    action: MenuAction,
    disabled: BoolFn,
) -> MenuEntry {
    MenuEntry {
        kind: MenuEntryKind::Item {
            label,
            shortcut,
            action,
            active: None,
            disabled: Some(disabled),
        },
    }
}

const fn sep() -> MenuEntry {
    MenuEntry {
        kind: MenuEntryKind::Separator,
    }
}

const fn submenu(id: &'static str, label: &'static str, items: &'static [MenuEntry]) -> MenuEntry {
    MenuEntry {
        kind: MenuEntryKind::Submenu {
            id,
            label,
            items,
        },
    }
}

// --- predicate helpers -----------------------------------------------------

fn view_pages(s: &AppState) -> bool {
    s.surface.view == ViewMode::Pages
}
fn view_continuous(s: &AppState) -> bool {
    s.surface.view == ViewMode::Continuous
}
fn outline_on(s: &AppState) -> bool {
    s.preferences.show_outline
}
fn ruler_on(s: &AppState) -> bool {
    s.preferences.show_ruler
}
fn focus_on(s: &AppState) -> bool {
    s.surface.focus_mode
}
fn not_ready(s: &AppState) -> bool {
    !s.surface.ready
}
fn unavailable(s: &AppState) -> bool {
    s.unavailable()
}
fn no_undo(s: &AppState) -> bool {
    !s.can_undo
}
fn no_redo(s: &AppState) -> bool {
    !s.can_redo
}
fn bookmarks_on(s: &AppState) -> bool {
    s.show_bookmarks
}
fn paragraph_breaks_on(s: &AppState) -> bool {
    s.show_paragraph_breaks
}

// --- menu definitions -------------------------------------------------------
// Order matches EditorWorkspace.tsx exactly.

const FILE_ITEMS: &[MenuEntry] = &[
    item("New", Some("Ctrl+N"), MenuAction::Command("newDocument")),
    item("Open", Some("Ctrl+O"), MenuAction::Command("open")),
    MenuEntry {
        kind: MenuEntryKind::RecentSubmenu,
    },
    sep(),
    item("Save", Some("Ctrl+S"), MenuAction::Command("save")),
    item("Export DOCX", Some("Ctrl+Shift+S"), MenuAction::Command("saveAs")),
    item("Export Plain Text", None, MenuAction::Command("exportPlainText")),
    sep(),
    item("Page Setup", None, MenuAction::Command("pageSetup")),
];

const EDIT_ITEMS: &[MenuEntry] = &[
    item_disabled(
        "Undo",
        Some("Ctrl+Z"),
        MenuAction::EditorExec(r#"{"type":"undo"}"#),
        no_undo,
    ),
    item_disabled(
        "Redo",
        Some("Ctrl+Y"),
        MenuAction::EditorExec(r#"{"type":"redo"}"#),
        no_redo,
    ),
    sep(),
    item(
        "Select All",
        Some("Ctrl+A"),
        MenuAction::EditorExec(r#"{"type":"selectAll"}"#),
    ),
    item("Find", Some("Ctrl+F"), MenuAction::Command("find")),
    item(
        "Find And Replace",
        Some("Ctrl+Shift+I"),
        MenuAction::Command("findReplace"),
    ),
];

const VIEW_ITEMS: &[MenuEntry] = &[
    item_active(
        "Separate Pages",
        None,
        MenuAction::CommandJson("setView", r#"{"view":"pages"}"#),
        view_pages,
    ),
    item_active(
        "Continuous",
        None,
        MenuAction::CommandJson("setView", r#"{"view":"continuous"}"#),
        view_continuous,
    ),
    sep(),
    item_active("Document Outline", None, MenuAction::ToggleOutline, outline_on),
    item_active("Show Ruler", None, MenuAction::ToggleRuler, ruler_on),
    item_active(
        "Show Paragraph Breaks",
        None,
        MenuAction::Command("paragraphMarks"),
        paragraph_breaks_on,
    ),
    item_active(
        "Show Bookmarks",
        None,
        MenuAction::Command("showBookmarks"),
        bookmarks_on,
    ),
    item_active(
        "Focus Mode",
        Some("Ctrl+Shift+F"),
        MenuAction::ToggleFocus,
        focus_on,
    ),
    sep(),
    item("Go To Page", Some("Ctrl+G"), MenuAction::Modal(Modal::GoToPage)),
];

// Insert preset items (library CHROME_MENUS) + custom items from
// EditorWorkspace.tsx. Verified editor.exec payloads from commandForSlot.
const BREAK_ITEMS: &[MenuEntry] = &[
    item(
        "Page Break",
        None,
        MenuAction::EditorExec(r#"{"type":"insertBreak","kind":"page"}"#),
    ),
    item(
        "Section Break (Next Page)",
        None,
        MenuAction::EditorExec(r#"{"type":"insertBreak","kind":"section"}"#),
    ),
    item(
        "Section Break (Continuous)",
        None,
        MenuAction::EditorExec(r#"{"type":"insertBreak","kind":"sectionContinuous"}"#),
    ),
];

const HEADER_FOOTER_ITEMS: &[MenuEntry] = &[
    item("Edit Header", None, MenuAction::Command("editHeader")),
    item("Edit Footer", None, MenuAction::Command("editFooter")),
    item(
        "Return To Document",
        None,
        MenuAction::Command("exitHeaderFooter"),
    ),
];

const PAGE_FIELD_ITEMS: &[MenuEntry] = &[
    item("Page Number", None, MenuAction::Command("pageNumber")),
    item("Total Pages", None, MenuAction::Command("totalPages")),
    item("Page X Of Y", None, MenuAction::Command("pageXofY")),
];

const INSERT_ITEMS: &[MenuEntry] = &[
    // Library preset items.
    item("Image", None, MenuAction::EditorExec(r#"{"type":"insertImage"}"#)),
    item("Text Box", None, MenuAction::EditorExec(r#"{"type":"insertTextBox"}"#)),
    MenuEntry {
        kind: MenuEntryKind::TableGrid,
    },
    sep(),
    item(
        "Insert Footnote",
        None,
        MenuAction::EditorExec(r#"{"type":"insertNote","noteKind":"footnote"}"#),
    ),
    item(
        "Insert Endnote",
        None,
        MenuAction::EditorExec(r#"{"type":"insertNote","noteKind":"endnote"}"#),
    ),
    sep(),
    submenu("insert.break", "Break", BREAK_ITEMS),
    item("Table Of Contents", None, MenuAction::EditorExec(r#"{"type":"insertToc"}"#)),
    item(
        "Bookmark",
        Some("Ctrl+Shift+F5"),
        MenuAction::EditorExec(r#"{"type":"openBookmarkDialog"}"#),
    ),
    item(
        "Go To Bookmark",
        Some("Ctrl+G"),
        MenuAction::EditorExec(r#"{"type":"openBookmarkDialog"}"#),
    ),
    item("Watermark", None, MenuAction::EditorExec(r#"{"type":"openWatermarkDialog"}"#)),
    // Custom items appended by EditorWorkspace.tsx.
    sep(),
    item("Date And Time", None, MenuAction::Command("insertDateTime")),
    item("Special Characters", None, MenuAction::Command("insertSymbols")),
    item("Math Equation", None, MenuAction::Command("insertMath")),
    item("Custom Table", None, MenuAction::Command("insertTable")),
    submenu("insert.headers", "Headers And Footers", HEADER_FOOTER_ITEMS),
    submenu("insert.pagefields", "Page Number Fields", PAGE_FIELD_ITEMS),
];

// Format preset items (library CHROME_MENUS) + custom Text Position submenu.
const TEXT_POSITION_ITEMS: &[MenuEntry] = &[
    item(
        "Superscript",
        None,
        MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"superscript"}"#),
    ),
    item(
        "Subscript",
        None,
        MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"subscript"}"#),
    ),
];

const FORMAT_ITEMS: &[MenuEntry] = &[
    item("Bold", Some("Ctrl+B"), MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"bold"}"#)),
    item("Italic", Some("Ctrl+I"), MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"italic"}"#)),
    item("Underline", Some("Ctrl+U"), MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"underline"}"#)),
    item("Strikethrough", None, MenuAction::EditorExec(r#"{"type":"toggleMark","mark":"strike"}"#)),
    sep(),
    item("Align Left", None, MenuAction::EditorExec(r#"{"type":"setAlignment","align":"left"}"#)),
    item("Center", None, MenuAction::EditorExec(r#"{"type":"setAlignment","align":"center"}"#)),
    item("Align Right", None, MenuAction::EditorExec(r#"{"type":"setAlignment","align":"right"}"#)),
    item("Justify", None, MenuAction::EditorExec(r#"{"type":"setAlignment","align":"justify"}"#)),
    sep(),
    item("Left-To-Right Text", None, MenuAction::EditorExec(r#"{"type":"setParagraphDirection","direction":"ltr"}"#)),
    item("Right-To-Left Text", None, MenuAction::EditorExec(r#"{"type":"setParagraphDirection","direction":"rtl"}"#)),
    sep(),
    item("Line Spacing Options…", None, MenuAction::EditorExec(r#"{"type":"openParagraphDialog"}"#)),
    sep(),
    item("Format Painter", Some("Ctrl+Alt+C"), MenuAction::EditorExec(r#"{"type":"copyFormatting"}"#)),
    item("Clear Formatting", None, MenuAction::EditorExec(r#"{"type":"clearFormatting"}"#)),
    // Custom items appended by EditorWorkspace.tsx.
    sep(),
    submenu("format.textpos", "Text Position", TEXT_POSITION_ITEMS),
];

const TOOLS_ITEMS: &[MenuEntry] = &[
    item_disabled(
        "Check Spelling",
        Some("Ctrl+Alt+X"),
        MenuAction::Command("checkSpelling"),
        |s| !s.preferences.check_spelling,
    ),
    item("Word Count", None, MenuAction::Modal(Modal::Statistics)),
    item("Keyboard Shortcuts", Some("Ctrl+/"), MenuAction::Modal(Modal::Shortcuts)),
    item(
        "Find And Replace",
        Some("Ctrl+Shift+I"),
        MenuAction::Command("findReplace"),
    ),
    sep(),
    item("Settings", Some("Ctrl+,"), MenuAction::Modal(Modal::Settings)),
];

const HELP_ITEMS: &[MenuEntry] = &[
    item("Search The Menus", Some("Alt+/"), MenuAction::Command("searchMenus")),
    item("Keyboard Shortcuts", Some("Ctrl+/"), MenuAction::Modal(Modal::Shortcuts)),
];

fn entries_for(menu: &str) -> &'static [MenuEntry] {
    match menu {
        "file" => FILE_ITEMS,
        "edit" => EDIT_ITEMS,
        "view" => VIEW_ITEMS,
        "insert" => INSERT_ITEMS,
        "format" => FORMAT_ITEMS,
        "tools" => TOOLS_ITEMS,
        "help" => HELP_ITEMS,
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Trigger row (called inside the header's menu row)
// ---------------------------------------------------------------------------

/// Draw the menu triggers. When the surface is not ready, the caller renders
/// the `.menu-placeholder` text instead.
pub fn draw_triggers(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
) {
    // Close an open menu when the user presses outside it (uses last frame's
    // popup rects; trigger clicks re-open below so they are unaffected).
    let popup_rects = ctx
        .data(|d| d.get_temp::<Vec<Rect>>(Id::new(MENU_POPUP_RECTS)))
        .unwrap_or_default();
    let trigger_rects = ctx
        .data(|d| d.get_temp::<Vec<Rect>>(Id::new(MENU_RECTS)))
        .unwrap_or_default();
    if state.open_menu.is_some() {
        if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
            let inside_popup = popup_rects.iter().any(|r| r.contains(pos));
            let inside_trigger = trigger_rects.iter().any(|r| r.contains(pos));
            if !inside_popup && !inside_trigger {
                state.open_menu = None;
                clear_all_submenus(ctx);
            }
        }
    }

    for (i, label) in MENU_LABELS.iter().enumerate() {
        let id = MENU_NAMES[i];
        let open = state.open_menu == Some(id);
        let font = FontId::proportional(13.0);
        let text_w = ui
            .painter()
            .layout_no_wrap(label.to_string(), font.clone(), c.app_text)
            .size()
            .x;
        let (rect, response) = ui.allocate_exact_size(
            vec2(text_w + 16.0, 26.0),
            Sense::click(),
        );
        ctx.data_mut(|d| {
            let mut v = d.get_temp::<Vec<Rect>>(Id::new(MENU_RECTS)).unwrap_or_default();
            // replace or push by index order — menus drawn in fixed order
            while v.len() <= i {
                v.push(Rect::NOTHING);
            }
            v[i] = rect;
            d.insert_temp(Id::new(MENU_RECTS), v);
        });
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                4.0,
                c.app_hover,
            );
        }
        if open {
            ui.painter().rect_filled(rect, 4.0, c.app_hover);
        }
        ui.painter().text(
            Pos2::new(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            *label,
            font,
            c.app_text,
        );
        if response.clicked() {
            if state.open_menu == Some(id) {
                state.open_menu = None;
            } else {
                state.open_menu = Some(id);
            }
            clear_all_submenus(ctx);
            if state.open_menu == Some("file") {
                maybe_fetch_recents(ctx, out);
            }
        }
    }
}

fn maybe_fetch_recents(ctx: &egui::Context, out: &mut UiOutput) {
    let last = ctx
        .data(|d| d.get_temp::<Instant>(Id::new(RECENT_FETCHED)))
        .unwrap_or_else(Instant::now)
        .elapsed();
    let have = ctx
        .data(|d| d.get_temp::<Vec<RecentEntry>>(Id::new(super::RECENTS_ID)))
        .is_some();
    if !have || last > Duration::from_secs(30) {
        out.commands.push(UiCommand::FetchRecent);
        ctx.data_mut(|d| d.insert_temp(Id::new(RECENT_FETCHED), Instant::now()));
    }
}

// ---------------------------------------------------------------------------
// Open popup rendering (called at end of frame so menus layer on top)
// ---------------------------------------------------------------------------

pub fn draw_open_menus(
    ctx: &egui::Context,
    state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
) {
    let Some(menu) = state.open_menu else {
        ctx.data_mut(|d| d.insert_temp(Id::new(MENU_POPUP_RECTS), Vec::<Rect>::new()));
        return;
    };
    let c = colors(assets);
    let trigger_rects = ctx
        .data(|d| d.get_temp::<Vec<Rect>>(Id::new(MENU_RECTS)))
        .unwrap_or_default();
    let idx = MENU_NAMES.iter().position(|m| *m == menu).unwrap_or(0);
    let anchor = trigger_rects.get(idx).copied().unwrap_or(Rect::NOTHING);
    if anchor == Rect::NOTHING {
        return;
    }

    let mut popup_rects: Vec<Rect> = Vec::new();
    let pos = Pos2::new(anchor.left(), anchor.bottom() + 2.0);

    let entries: &'static [MenuEntry] = entries_for(menu);
    let popup_id = Id::new(("menu_popup", menu));
    let inner = egui::Area::new(popup_id)
        .order(egui::Order::Foreground)
        .pivot(Align2::LEFT_TOP)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .fill(c.app_surface)
                .stroke(Stroke::new(1.0_f32, c.app_border))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(5))
                .show(ui, |ui| {
                    ui.set_min_width(170.0);
                    render_entries(ui, ctx, state, &c, out, entries, menu, 0);
                })
        });
    popup_rects.push(inner.response.rect);

    // Render the open submenu (one level of nesting is all the menus use,
    // but recursion supports more).
    render_submenus(ctx, state, assets, out, menu, &c, &mut popup_rects, 0);

    // Close the whole menu when the pointer is outside every open popup and
    // every trigger (menus close on mouse-out like the CSS version).
    if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
        let in_popup = popup_rects.iter().any(|r| r.contains(pos));
        let in_trigger = trigger_rects.iter().any(|r| r.contains(pos));
        if !in_popup && !in_trigger {
            state.open_menu = None;
            clear_all_submenus(ctx);
        }
    }

    ctx.data_mut(|d| d.insert_temp(Id::new(MENU_POPUP_RECTS), popup_rects));
}

fn render_submenus(
    ctx: &egui::Context,
    state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
    top_menu: &str,
    c: &UiColors,
    popup_rects: &mut Vec<Rect>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    // Collect open submenus for this level. Submenu state is stored per
    // top menu — only one submenu can be open at a time.
    let open_sub = ctx.data(|d| d.get_temp::<OpenSub>(Id::new(("submenu_open", top_menu))));
    let Some(open) = open_sub else { return };
    let anchor = open.anchor;
    if anchor == Rect::NOTHING {
        return;
    }
    let pos = Pos2::new(anchor.right() + 1.0, anchor.top() - 5.0);
    let popup_id = Id::new(("submenu_popup", top_menu, open.id.clone()));

    // Dynamic submenus (Open Recent, Table grid) render custom content.
    if find_dynamic_items(top_menu, &open.id).is_some() {
        let inner = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .pivot(Align2::LEFT_TOP)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                Frame::popup(ui.style())
                    .fill(c.app_surface)
                    .stroke(Stroke::new(1.0_f32, c.app_border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(Margin::same(5))
                    .show(ui, |ui| {
                        ui.set_min_width(170.0);
                        render_dynamic_submenu(ui, ctx, state, c, out, top_menu, &open.id, depth + 1);
                    })
            });
        popup_rects.push(inner.response.rect);
        return;
    }

    // Find the entries for the static submenu by id.
    let Some(items) = find_submenu_items(top_menu, &open.id) else {
        ctx.data_mut(|d| d.remove::<OpenSub>(Id::new(("submenu_open", top_menu))));
        return;
    };
    let inner = egui::Area::new(popup_id)
        .order(egui::Order::Foreground)
        .pivot(Align2::LEFT_TOP)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .fill(c.app_surface)
                .stroke(Stroke::new(1.0_f32, c.app_border))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(Margin::same(5))
                .show(ui, |ui| {
                    ui.set_min_width(170.0);
                    render_entries(ui, ctx, state, c, out, items, top_menu, depth + 1);
                })
        });
    popup_rects.push(inner.response.rect);
    render_submenus(ctx, state, assets, out, top_menu, c, popup_rects, depth + 1);
}

#[derive(Clone)]
pub(crate) struct OpenSub {
    pub id: String,
    pub anchor: Rect,
}

/// Remove all open submenu state (called when a menu closes).
pub fn clear_submenus(ctx: &egui::Context) {
    for m in MENU_NAMES {
        ctx.data_mut(|d| {
            d.remove::<OpenSub>(Id::new(("submenu_open", m)));
        });
    }
}

fn find_submenu_items<'a>(top: &str, id: &str) -> Option<&'static [MenuEntry]> {
    // Submenu ids are unique across the whole menu bar.
    fn search(entries: &'static [MenuEntry], id: &str) -> Option<&'static [MenuEntry]> {
        for e in entries {
            if let MenuEntryKind::Submenu { id: sid, items, .. } = e.kind {
                if sid == id {
                    return Some(items);
                }
                if let Some(found) = search(items, id) {
                    return Some(found);
                }
            }
        }
        None
    }
    search(entries_for(top), id)
}

// ---------------------------------------------------------------------------
// Entry rendering
// ---------------------------------------------------------------------------

fn render_entries(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
    entries: &[MenuEntry],
    top_menu: &str,
    depth: usize,
) {
    for entry in entries {
        match entry.kind {
            MenuEntryKind::Separator => {
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), 9.0),
                    Sense::hover(),
                );
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(1.0_f32, c.app_border),
                );
            }
            MenuEntryKind::Item {
                label,
                shortcut,
                action,
                active,
                disabled,
            } => {
                let is_disabled = disabled.map(|f| f(state)).unwrap_or(false);
                let is_active = active.map(|f| f(state)).unwrap_or(false);
                let response = menu_item_row(ui, c, label, shortcut, is_active, is_disabled, false);
                if response.clicked() && !is_disabled {
                    fire_action(state, out, action);
                    state.open_menu = None;
                    clear_all_submenus(ctx);
                }
            }
            MenuEntryKind::Submenu { id, label, .. } => {
                let response = menu_item_row(ui, c, label, None, false, false, true);
                if response.hovered() && depth == 0 {
                    // Open (or keep) this submenu; store anchor for popup.
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            Id::new(("submenu_open", top_menu)),
                            OpenSub {
                                id: id.to_string(),
                                anchor: response.rect,
                            },
                        )
                    });
                }
            }
            MenuEntryKind::RecentSubmenu => {
                let response = menu_item_row(ui, c, "Open Recent", None, false, false, true);
                if response.hovered() && depth == 0 {
                    maybe_fetch_recents(ctx, out);
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            Id::new(("submenu_open", top_menu)),
                            OpenSub {
                                id: "file.recent".to_string(),
                                anchor: response.rect,
                            },
                        )
                    });
                }
            }
            MenuEntryKind::TableGrid => {
                let response = menu_item_row(ui, c, "Table", None, false, false, true);
                if response.hovered() && depth == 0 {
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            Id::new(("submenu_open", top_menu)),
                            OpenSub {
                                id: "insert.tablegrid".to_string(),
                                anchor: response.rect,
                            },
                        )
                    });
                }
            }
        }
    }
}

/// Override `find_submenu_items` to also resolve the dynamic entries.
fn find_dynamic_items(top: &str, id: &str) -> Option<DynamicItems> {
    if id == "file.recent" && top == "file" {
        return Some(DynamicItems::Recent);
    }
    if id == "insert.tablegrid" && top == "insert" {
        return Some(DynamicItems::TableGrid);
    }
    None
}

#[derive(Clone, Copy)]
enum DynamicItems {
    Recent,
    TableGrid,
}

/// One menu row: label left, optional shortcut right, optional caret.
/// Returns the response; paints hover/active states.
fn menu_item_row(
    ui: &mut egui::Ui,
    c: &UiColors,
    label: &str,
    shortcut: Option<&str>,
    active: bool,
    disabled: bool,
    caret: bool,
) -> egui::Response {
    let height = 30.0;
    let width = ui.available_width().max(170.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hovered = response.hovered() && !disabled;
    if hovered {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), c.app_hover);
    }
    let text_color = if disabled {
        dim(c.app_text, 0.4)
    } else if active {
        c.app_accent_ink
    } else {
        c.app_text
    };
    let font = FontId::proportional(13.0);
    ui.painter().text(
        Pos2::new(rect.left() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        text_color,
    );
    let mut right = rect.right() - 10.0;
    if let Some(sc) = shortcut {
        let sc_font = FontId::proportional(12.0);
        let sc_w = ui
            .painter()
            .layout_no_wrap(sc.to_string(), sc_font.clone(), c.app_muted)
            .size()
            .x;
        ui.painter().text(
            Pos2::new(right - sc_w, rect.center().y),
            Align2::LEFT_CENTER,
            sc,
            sc_font,
            if disabled { dim(c.app_muted, 0.4) } else { c.app_muted },
        );
        right -= sc_w + 12.0;
    }
    if caret {
        ui.painter().text(
            Pos2::new(right - 8.0, rect.center().y),
            Align2::CENTER_CENTER,
            "›",
            font,
            if disabled { dim(c.app_muted, 0.4) } else { c.app_muted },
        );
    }
    response
}

fn dim(color: Color32, f: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * f) as u8,
        (color.g() as f32 * f) as u8,
        (color.b() as f32 * f) as u8,
        color.a(),
    )
}

// ---------------------------------------------------------------------------
// Dynamic submenu contents
// ---------------------------------------------------------------------------

/// Called from render_submenus when the open submenu is a dynamic one.
/// Returns true if it handled rendering.
pub fn render_dynamic_submenu(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
    top_menu: &str,
    open_id: &str,
    _depth: usize,
) -> bool {
    match find_dynamic_items(top_menu, open_id) {
        Some(DynamicItems::Recent) => {
            let items = recents(ctx);
            if items.is_empty() {
                menu_item_row(ui, c, "No Recent Documents", None, false, true, false);
            } else {
                for entry in &items {
                    recent_row(ui, ctx, c, entry, state, out);
                }
                ui.add_space(4.0);
                let (rect, response) = ui.allocate_exact_size(
                    vec2(ui.available_width().max(170.0), 30.0),
                    Sense::click(),
                );
                if response.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(5),
                        c.app_hover,
                    );
                }
                ui.painter().text(
                    Pos2::new(rect.left() + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    "Clear Recent Files",
                    FontId::proportional(13.0),
                    c.app_text,
                );
                if response.clicked() {
                    out.command("clearRecent");
                    state.open_menu = None;
                    clear_all_submenus(ctx);
                }
            }
            true
        }
        Some(DynamicItems::TableGrid) => {
            draw_table_grid(ui, ctx, state, c, out);
            true
        }
        None => false,
    }
}

fn recent_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    c: &UiColors,
    entry: &RecentEntry,
    state: &mut AppState,
    out: &mut UiOutput,
) {
    let width = ui.available_width().max(230.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 46.0), Sense::click());
    let hovered = response.hovered() && entry.available;
    if hovered {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(5),
            c.app_hover,
        );
    }
    let name_color = if entry.available { c.app_text } else { dim(c.app_text, 0.4) };
    let path_color = if entry.available { c.app_muted } else { dim(c.app_muted, 0.4) };
    ui.painter().text(
        Pos2::new(rect.left() + 10.0, rect.top() + 13.0),
        Align2::LEFT_TOP,
        truncate(&entry.name, 34),
        FontId::proportional(12.0),
        name_color,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 10.0, rect.top() + 29.0),
        Align2::LEFT_TOP,
        truncate(&entry.path, 44),
        FontId::proportional(10.0),
        path_color,
    );
    if response.clicked() && entry.available {
        out.command_args(
            "openRecent",
            serde_json::json!({ "token": entry.token }),
        );
        state.open_menu = None;
        clear_all_submenus(ctx);
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Insert > Table picker: 6 rows x 6 columns grid (Pu=6, _b=6 in the lib).
fn draw_table_grid(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
) {
    const COLS: usize = 6;
    const ROWS: usize = 6;
    let cell = 18.0;
    let hover_cell = ctx
        .data(|d| d.get_temp::<(usize, usize)>(Id::new("table_grid_hover")))
        .unwrap_or((0, 0));
    let mut clicked: Option<(usize, usize)> = None;
    for row in 0..ROWS {
        ui.horizontal(|ui| {
            for col in 0..COLS {
                let (rect, response) = ui.allocate_exact_size(
                    vec2(cell, cell),
                    Sense::click(),
                );
                let active = row < hover_cell.0 && col < hover_cell.1 && hover_cell.0 > 0;
                if active {
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        c.app_accent_soft,
                    );
                }
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(1.0_f32, c.app_border),
                    egui::StrokeKind::Inside,
                );
                if response.hovered() {
                    ctx.data_mut(|d| {
                        d.insert_temp(Id::new("table_grid_hover"), (row + 1, col + 1))
                    });
                }
                if response.clicked() {
                    clicked = Some((row + 1, col + 1));
                }
            }
        });
    }
    ui.add_space(2.0);
    let (cap_rect, _) = ui.allocate_exact_size(
        vec2(cell * COLS as f32, 18.0),
        Sense::hover(),
    );
    ui.painter().text(
        cap_rect.center(),
        Align2::CENTER_CENTER,
        if hover_cell.0 > 0 && hover_cell.1 > 0 {
            format!("{} × {}", hover_cell.1, hover_cell.0)
        } else {
            String::new()
        },
        FontId::proportional(11.0),
        c.app_muted,
    );
    if let Some((rows, cols)) = clicked {
        out.exec(
            "editorExec",
            serde_json::json!({ "type": "insertTable", "rows": rows, "cols": cols }),
        );
        state.open_menu = None;
        clear_all_submenus(ctx);
    }
}

// ---------------------------------------------------------------------------
// Action dispatch
// ---------------------------------------------------------------------------

fn fire_action(state: &mut AppState, out: &mut UiOutput, action: MenuAction) {
    match action {
        MenuAction::Command(name) => out.command(name),
        MenuAction::CommandJson(name, args_json) => {
            let args: Value = serde_json::from_str(args_json).unwrap_or(Value::Null);
            out.command_args(name, args);
        }
        MenuAction::EditorExec(cmd_json) => {
            let args: Value = serde_json::from_str(cmd_json).unwrap_or(Value::Null);
            out.exec("editorExec", args);
        }
        MenuAction::Exec(name, args_json) => {
            let args: Value = serde_json::from_str(args_json).unwrap_or(Value::Null);
            out.exec(name, args);
        }
        MenuAction::Modal(m) => {
            state.modal = m;
        }
        MenuAction::ToggleOutline => {
            state.preferences.show_outline = !state.preferences.show_outline;
            out.commands.push(UiCommand::SavePreferences);
        }
        MenuAction::ToggleRuler => {
            state.preferences.show_ruler = !state.preferences.show_ruler;
            out.commands.push(UiCommand::SavePreferences);
        }
        MenuAction::ToggleFocus => {
            let enabled = !state.surface.focus_mode;
            out.command_args("focusMode", serde_json::json!({ "enabled": enabled }));
        }
    }
}
