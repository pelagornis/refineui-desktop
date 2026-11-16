#!/usr/bin/env python3
"""Generate the component / foundation pages in docs/src/content/docs/.

Page text (summary, usage notes, Rust example) is defined below with `page(...)`;
the API tables are extracted from the `///` docs and `pub fn` signatures in
crates/refineui/src. Page order follows the gallery stories.

  python3 scripts/gen_docs.py

Edit this file, not the generated .mdx files under components/ and foundations/.
"""
import re, os, json, pathlib
ROOT=str(pathlib.Path(__file__).resolve().parent.parent)
SRC=ROOT+'/crates/refineui/src/'
OUT=ROOT+'/docs/src/content/docs/'

def parse_file(path):
    lines=open(path).read().split('\n')
    types={}
    i=0
    doc=[]
    while i<len(lines):
        l=lines[i]; s=l.strip()
        if s.startswith('///'):
            doc.append(s[3:].strip()); i+=1; continue
        if s.startswith('#['): i+=1; continue
        m=re.match(r'pub (struct|enum|type) (\w+)',s)
        if m and not l.startswith(' '):
            kind,name=m.groups()
            t=types.setdefault(name,{'kind':kind,'doc':' '.join(doc),'variants':[],'methods':[]})
            t['doc']=' '.join(doc) or t['doc']; t['kind']=kind
            if kind=='enum':
                vdoc=[]; i+=1
                while not lines[i].startswith('}'):
                    v=lines[i].strip()
                    if v.startswith('///'): vdoc.append(v[3:].strip())
                    elif re.match(r'^[A-Z]\w*(\(.*\))?,$',v): t['variants'].append((v.rstrip(','),' '.join(vdoc))); vdoc=[]
                    i+=1
            doc=[]; i+=1; continue
        m=re.match(r'^impl(?:<[^>]*>)? (\w+) \{',l)
        if m:
            name=m.group(1); t=types.setdefault(name,{'kind':'struct','doc':'','variants':[],'methods':[]})
            i+=1; mdoc=[]
            while i<len(lines) and not lines[i].startswith('}'):
                v=lines[i].strip()
                if v.startswith('///'): mdoc.append(v[3:].strip())
                elif v.startswith('pub fn') or v.startswith('pub const'):
                    sig=v
                    while not (sig.rstrip().endswith('{') or sig.rstrip().endswith(';')) and i+1<len(lines):
                        i+=1; sig+=' '+lines[i].strip()
                    sig=re.sub(r'\s+',' ',sig).rstrip('{; ').replace('( ','(').replace(', )',')')
                    if v.startswith('pub fn'):
                        t['methods'].append((sig[4:],' '.join(mdoc)))
                    mdoc=[]
                elif v and not v.startswith('#['): mdoc=[] if not v.startswith('//') else mdoc
                i+=1
        doc=[]; i+=1
    return types

def esc(s):
    s=re.sub(r'\[`([^`]+)`\](\([^)]*\))?', r'`\1`', s)
    parts=s.split('`')
    for i in range(0,len(parts),2):
        parts[i]=parts[i].replace('{','&#123;').replace('}','&#125;').replace('<','&lt;').replace('>','&gt;')
    return '`'.join(parts).replace('|','\\|')
def code(s): return s.replace('|','\\|')

def api_md(file, names):
    types=parse_file(SRC+file)
    out=[]
    for n in names:
        # `other/file.rs:Type` pulls a type from another source file.
        if ':' in n:
            f,n=n.split(':')
            t=parse_file(SRC+f).get(n)
        else:
            t=types.get(n)
        if not t: raise SystemExit(f'missing {n} in {file}')
        out.append(f'### {n}\n')
        if t['doc']: out.append(esc(t['doc'])+'\n')
        if t['variants']:
            out.append('| Variant | Description |\n| --- | --- |')
            out+= [f'| `{code(v)}` | {esc(d)} |' for v,d in t['variants']]
            out.append('')
        if t['methods']:
            out.append('| Method | Description |\n| --- | --- |')
            out+= [f'| `{code(sig)}` | {esc(d)} |' for sig,d in t['methods']]
            out.append('')
    return '\n'.join(out)

# id: (title, group, file, types, description, usage notes, example, order)
P={}
def page(id,title,group,file,types,desc,notes,example): P[id]=(title,group,file,types,desc,notes,example)

page('button','Button','actions','components/button.rs',['Button','ButtonVariant'],
 'Primary, secondary, outline and ghost buttons in three sizes, with icons, loading and disabled states.',
 ['Buttons are focusable with Tab and activate on click, Enter or Space.','Use `Button::icon_only` for toolbar actions and always give it a `tooltip`.','`loading(true)` swaps the leading icon for a spinner and blocks clicks.'],
 '''Button::new("save", "Save")
    .variant(ButtonVariant::Primary)
    .icon(IconName::Checkmark)
    .on_click(|_, _window, _cx| println!("saved"));

Button::icon_only("settings", IconName::Settings)
    .variant(ButtonVariant::Ghost)
    .tooltip("Settings");''')
page('toggle-button','Toggle Button','actions','components/toggle.rs',['ToggleButton'],'A two-state toolbar button; pressed uses the selected plate.',
 ['ToggleButton is controlled: pass the current state and update it in `on_change`.','The web `Toggle` (an on/off switch) is `Switch` here; `Toggle` is kept as an alias of `Switch`.'],
 '''ToggleButton::new("bold", "Bold", self.bold)
    .on_change(cx.listener(|this, on: &bool, _, cx| {
        this.bold = *on;
        cx.notify();
    }));''')
page('segmented-control','Segmented Control','actions','components/segmented_control.rs',['SegmentedControl','Segment'],'Mutually exclusive segments for switching views or modes.',
 ['Segments accept plain strings or `Segment::with_icon`.','Arrow keys move the selection when the control is focused.'],
 '''SegmentedControl::new("view", [
    Segment::with_icon("List", IconName::Document),
    Segment::with_icon("Grid", IconName::Grid),
], self.view)
.on_change(cx.listener(|this, ix: &usize, _, cx| {
    this.view = *ix;
    cx.notify();
}));''')
page('link','Link','actions','components/link.rs',['Link'],'Inline text link that opens a URL or runs a handler.',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','With `href` the link opens the URL in the default browser via `cx.open_url`.'],
 '''Link::new("docs", "Read the docs").href("https://github.com/pelagornis/refineui-gpui");
Link::new("gpui", "GPUI").href("https://gpui.rs").external(true);''')
page('input','Input','forms','components/input.rs',['TextInput','InputEvent'],'Single-line text input with selection, clipboard, IME, placeholder, search and password modes.',
 ['`TextInput` is an entity: create it once with `cx.new(|cx| TextInput::new(cx))` and keep it in your view.','Subscribe to `InputEvent::Change` / `InputEvent::Submit` with `cx.subscribe`.','Key bindings (arrows, Home/End, ⌘A, ⌘C/⌘X/⌘V, Enter, Escape) live in the `RefineUITextInput` key context.'],
 '''let name = cx.new(|cx| TextInput::new(cx).placeholder("Project name"));
cx.subscribe(&name, |this, input, event: &InputEvent, cx| {
    if let InputEvent::Submit = event {
        this.create(input.read(cx).text(), cx);
    }
})
.detach();

// in render():
div().w(px(240.)).child(self.name.clone())''')
page('textarea','Textarea','forms','components/input.rs',['Textarea'],'Multi-line text input with soft wrap (web `Textarea`).',
 ['`Textarea::new(cx)` returns a multi-line `TextInput`; Enter inserts a newline.','Body text is `bodyMd`, the shell is `roundedLarge` with an 80px minimum height.'],
 '''let notes = cx.new(|cx| Textarea::new(cx).placeholder("Write a message…"));

// in render():
div().w(px(320.)).child(self.notes.clone())''')
page('search-field','Search Field','forms','components/input.rs',['SearchField','InputAppearance'],'Filled search input with a leading search icon and clear button (web `SearchField`).',
 ['`SearchField::new(cx)` returns a filled `TextInput`: `backgroundSurfaceHover` plate, `backgroundSurfaceActive` on hover, a thin `borderDefault` while focused.','Escape clears the text; the leading icon is small / medium / large per size.'],
 '''let search = cx.new(|cx| SearchField::new(cx).placeholder("Search components"));
cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
    if let InputEvent::Change(query) = event {
        this.filter(query, cx);
    }
})
.detach();''')
page('input-otp','Input OTP','forms','components/input_otp.rs',['InputOtp'],'One-time-code input made of independent soft square cells.',
 ['Cells are `controlHeight` squares (36 / 44 / 52) with title-scale digits.','Digits fill the next empty cell, Backspace clears, ⌘V / Ctrl-V pastes; `on_complete` fires once every cell is filled.'],
 '''InputOtp::new("code")
    .length(6)
    .on_complete(cx.listener(|this, code: &SharedString, _, cx| this.verify(code, cx)));''')
page('field','Field','forms','components/field.rs',['Field'],'Label, hint, required marker and validation message around any control.',
 ['Show either a hint or an error; `error` replaces the hint and tints the message.','Mark the inner `TextInput` invalid too so its border matches.'],
 '''Field::new()
    .label("Email")
    .required(true)
    .hint("We never share it.")
    .error_opt(self.email_error.clone())
    .control(self.email.clone());''')
page('checkbox','Checkbox','forms','components/checkbox.rs',['Checkbox','CheckState'],'Checked, unchecked and indeterminate checkboxes with optional label and description.',
 ['Pass a `bool` or a `CheckState`; clicking an indeterminate box reports `true`.'],
 '''Checkbox::new("sync", self.sync)
    .label("Sync settings")
    .description("Keep preferences in sync across devices.")
    .on_change(cx.listener(|this, on: &bool, _, cx| {
        this.sync = *on;
        cx.notify();
    }));''')
page('radio','Radio','forms','components/radio.rs',['RadioGroup','Radio'],'Single choice from a small set of options.',
 ['`RadioGroup` handles arrow-key navigation; use `Radio` directly for custom layouts.'],
 '''RadioGroup::new("theme", ["System", "Light", "Dark"], Some(self.theme))
    .on_change(cx.listener(|this, ix: &usize, _, cx| {
        this.theme = *ix;
        cx.notify();
    }));''')
page('switch','Switch','forms','components/switch.rs',['Switch'],'Immediate on/off setting with an animated thumb.',
 ['Use a switch when the change applies immediately; use a checkbox inside forms that are submitted.'],
 '''Switch::new("wifi", self.wifi)
    .label("Wi-Fi")
    .on_change(cx.listener(|this, on: &bool, _, cx| {
        this.wifi = *on;
        cx.notify();
    }));''')
page('slider','Slider','forms','components/slider.rs',['Slider'],'Pick a value from a continuous or stepped range by dragging, clicking or with the keyboard.',
 ['Arrow keys move by one step, Page Up/Down by ten, Home/End jump to the bounds.'],
 '''Slider::new("volume", self.volume)
    .range(0., 100.)
    .step(1.)
    .on_change(cx.listener(|this, v: &f32, _, cx| {
        this.volume = *v;
        cx.notify();
    }));''')
page('spin-button','Spin Button','forms','components/spin_button.rs',['SpinButton'],'Numeric stepper with increment / decrement buttons and keyboard support.',
 ['Values are clamped to `range` and rounded to `precision` decimals.'],
 '''SpinButton::new("delay", self.delay)
    .range(0., 10.)
    .step(0.25)
    .precision(2)
    .suffix(" s")
    .on_change(cx.listener(|this, v: &f64, _, cx| {
        this.delay = *v;
        cx.notify();
    }));''')
page('select','Select','forms','components/select.rs',['Select','SelectAppearance'],'Pick one option from a popup list.',
 ['Open with click, Enter, Space or ↓; navigate with arrows; Escape closes.'],
 '''Select::new("font", ["Inter", "SF Pro", "Menlo"], self.font)
    .placeholder("Font")
    .width(px(180.))
    .on_change(cx.listener(|this, ix: &usize, _, cx| {
        this.font = Some(*ix);
        cx.notify();
    }));''')
page('text','Text','display','components/text.rs',['Text','TextRole','TextVariant','TextColor','Heading','Label'],'Typography primitives mapped to the RefineUI type scale and semantic roles.',
 ['Prefer `TextRole` (bodyLg, labelMd, captionSm, titleMd …) — the same semantic roles as the web `Text` component.','`TextVariant` mirrors the raw Foundation tokens (heading1…caption3).'],
 '''Text::new("Settings").role(TextRole::TitleMd);
Heading::new(2, "Settings");
Text::new("Changes are saved automatically.").color(TextColor::Secondary);
Label::new("Name").required(true);''')
page('badge','Badge','display','components/badge.rs',['Badge','BadgeVariant'],'Small status or count indicator.',
 ['`Badge::count(n)` caps the label at `99+`.','Give a badge an `id` to make it interactive (hover / pressed plates per variant).'],
 '''Badge::new("Beta").variant(BadgeVariant::Warning);
Badge::count(unread).variant(BadgeVariant::Danger);''')
page('chip','Chip','display','components/chip.rs',['Chip','ChipVariant'],'Compact label that can be selected, clicked or removed. `Tag` is an alias.',
 ['`on_remove` adds a dismiss button whose glyph brightens on hover.','`Chip::tag` is an alias of `Chip::new` (the web Tag shares the chip anatomy).'],
 '''Chip::new("open", "Open").selected(self.filter == Filter::Open);
Chip::tag("rust", "rust").on_remove(|_, _, _| {});''')
page('avatar','Avatar','display','components/avatar.rs',['Avatar','AvatarSize','AvatarColor','AvatarLayout','Presence','Avatars','AvatarsLayout'],'Person initials, icon or image with presence glyphs, and stacked / spread groups.',
 ['Nine sizes from 16 to 60px (default 32). Initials use the neutral palette unless a `color` is set.','Presence draws the web glyphs: online dot, away crescent, unavailable bar, offline dot.'],
 '''Avatar::new("Grace Hopper").size(AvatarSize::Large).presence(Presence::Online);
Avatars::new(["Ada", "Grace", "Alan", "Linus"]).max(3);''')
page('card','Card','display','components/card.rs',['Card','CardVariant','CardState'],'Grouped content surface with optional header, action and footer.',
 ['Elevated (default): hairline `borderSubtle` + `shadow-8`. Outlined: thin `borderDefault`, no shadow.','`state(CardState::Hover | Pressed | Disabled)` forces the web `state` fills (`primaryHover`, `surfaceActive`, `surfaceDisabled`).','Children go into the card body; `flush()` removes body padding for tables and lists.','Give a card an `id` + `on_click` to make it interactive; `selected(true)` draws the 2px focus-colored ring.'],
 '''Card::new()
    .title("Storage")
    .description("62% of 1 TB used")
    .child(Progress::new("storage", 0.62))
    .footer(Button::new("manage", "Manage"));''')
page('alert','Alert','display','components/alert.rs',['Alert','AlertVariant'],'Inline message banner for info, success, warning and danger states.',
 ['Default variant is `Info`. Add an `action` button or an `on_close` dismiss handler.'],
 '''Alert::new("offline", "You are offline")
    .variant(AlertVariant::Warning)
    .description("Changes will sync when you reconnect.");''')
page('divider','Divider','display','components/divider.rs',['Divider','Axis','DividerAlign'],'Horizontal and vertical separators, optionally labelled or with a centered icon.',['Labels use `captionMd` in `foregroundBrand`; `align` moves the label to either end.'],
 '''Divider::horizontal().label("or");
h_stack(8.).h(px(20.)).child("A").child(Divider::vertical()).child("B");''')
page('kbd','Kbd','display','components/kbd.rs',['Kbd'],'Keyboard shortcut hint.',['Use `inverted(true)` on dark surfaces such as tooltips.'],
 '''Kbd::new("⌘K");''')
page('spinner','Spinner','display','components/spinner.rs',['Spinner','SpinnerSize'],'Indeterminate activity indicator.',[],
 '''Spinner::new().size(SpinnerSize::Medium).label("Loading…");''')
page('progress','Progress','display','components/progress.rs',['Progress'],'Determinate and indeterminate progress bars.',['For step-by-step completion use Progress Stepper.'],
 '''Progress::new("upload", 0.3).label("Uploading").show_value(true);
Progress::indeterminate("indexing");''')
page('progress-stepper','Progress Stepper','display','components/progress_stepper.rs',['ProgressStepper','StepState'],'Multi-step process completion indicator.',
 ['A 16px pill track whose fill ends just past the current pip; pips are inset 8px from each end.','Steps before `current` are complete. With `on_change` steps are clickable and focusable (Enter or Space activates); `disabled_step` dims a step and makes it inert.'],
 '''ProgressStepper::new("checkout", ["Cart", "Shipping", "Payment", "Review"], self.step)
    .on_change(cx.listener(|this, ix: &usize, _, cx| {
        this.step = *ix;
        cx.notify();
    }));''')
page('stepper','Stepper','display','components/stepper.rs',['Stepper'],'Numbered steps with checkmarks and animated connecting rails.',
 ['Steps before `current` are complete; clicking a step reports its index through `on_change`.'],
 '''Stepper::new("checkout", self.step)
    .step_with_description("Account", "Create login")
    .step("Billing")
    .step("Done")
    .on_change(cx.listener(|this, ix: &usize, _, cx| {
        this.step = *ix;
        cx.notify();
    }));''')
page('skeleton','Skeleton','display','components/skeleton.rs',['Skeleton','SkeletonShape'],'Pulsing loading placeholder shapes.',[],
 '''Skeleton::new().shape(SkeletonShape::Text).width(px(240.));''')
page('tooltip','Tooltip','display','components/tooltip.rs',['Tooltip'],'Hover hint with an optional shortcut.',
 ['Attach to any stateful element with `.tooltip(Tooltip::text("…"))`; `Button` and `Toggle` have a `.tooltip(text)` shorthand.'],
 '''div().id("save").child("Save").tooltip(Tooltip::with_shortcut("Save", "⌘S"));''')
page('toast','Toast','display','components/toast.rs',['Toaster','ToastData','ToastVariant','Toast'],'Transient notifications stacked at the top centre of the window.',
 ['Create one `Toaster` entity per window, render it as the last child of your root, and `push` toasts into it.','Newest toast is in front; hover the stack to fan it out. Toasts without a `duration` get a dismiss button.'],
 '''let toaster = cx.new(|_| Toaster::new());
toaster.update(cx, |t, cx| {
    t.push(ToastData::new("Export complete").variant(ToastVariant::Success), cx)
});

// in render():
refineui::root(cx).child(content).child(self.toaster.clone())''')
page('tabs','Tabs','navigation','components/tabs.rs',['Tabs','Tab'],'Switch between views in the same context.',[],
 '''Tabs::new("tabs", [Tab::new("Overview"), Tab::new("Issues").count(12)], self.tab)
    .on_change(cx.listener(|this, ix: &usize, _, cx| {
        this.tab = *ix;
        cx.notify();
    }));''')
page('breadcrumb','Breadcrumb','navigation','components/breadcrumb.rs',['Breadcrumb'],'Shows the location within a hierarchy; earlier items are clickable.',[],
 '''Breadcrumb::new("path", ["Workspace", "crates", "button.rs"])
    .on_select(|ix, _, _| println!("go to {ix}"));''')
page('pagination','Pagination','navigation','components/pagination.rs',['Pagination'],'Move between pages of results.',['Pages are 1-based; long ranges collapse with ellipses.'],
 '''Pagination::new("pages", self.page, 20)
    .on_change(cx.listener(|this, p: &usize, _, cx| {
        this.page = *p;
        cx.notify();
    }));''')
page('sidebar','Sidebar','navigation','components/sidebar.rs',['Sidebar','SidebarItem','SidebarPeek'],'App navigation landmark with a brand header, scrolling groups, footer, trigger and edge peek.',
 ['Inside a `ResizablePanel` use `.fill()`; standalone, `.width(px)` + `.collapsed(bool)` animates the width to 0 with the rail frozen.',
  '`sidebar_trigger(id, open, on_toggle)` is the web `SidebarTrigger` (secondary, `panel-left`).',
  '`SidebarPeek` floats a frosted copy in from the left edge while the sidebar is collapsed (hover the 12px edge for `duration-medium`).',
  'The content region is a `ScrollArea`, so group rows get the edge blur; footer rows do not.'],
 '''ResizablePanelGroup::new("app")
    .panel(
        ResizablePanel::new().size(32.).min_size(0.).max_size(48.).child(
            Sidebar::new("nav")
                .fill()
                .brand("RefineUI")
                .group(Some("Product"), [SidebarItem::new("overview", "Overview").icon(IconName::Search)])
                .footer_item(SidebarItem::new("account", "Pelagornis").icon(IconName::Person))
                .selected(self.route.clone())
                .on_select(cx.listener(|this, id: &SharedString, _, cx| {
                    this.route = id.clone();
                    cx.notify();
                })),
        ),
    )
    .handle(ResizableHandle::new().with_handle(true))
    .panel(ResizablePanel::new().size(68.).min_size(30.).child(main))''')
page('tree','Tree','navigation','components/tree.rs',['Tree','TreeNode'],'Expandable hierarchical list (file trees, outlines).',['Expansion state is kept internally; seed it with `default_expanded`.','Sizes map to `bodySm / bodyMd / bodyLg`; each depth indents by 22px.'],
 '''Tree::new("files", vec![
    TreeNode::branch("src", "src", vec![TreeNode::leaf("lib", "lib.rs")]),
])
.default_expanded(["src"]);''')
page('accordion','Accordion','navigation','components/accordion.rs',['Accordion'],'Stacked disclosure sections; the open item gets a soft plate.',['Only one item is open at a time unless `multiple(true)`.','`item_with_icon` adds a leading icon; sizes change trigger and content typography.'],
 '''Accordion::new("faq")
    .item("Does it support dark mode?", |_, _| Text::new("Yes.").into_any_element())
    .default_open([0]);''')
page('collapsible','Collapsible','navigation','components/accordion.rs',['Collapsible'],'Headless show / hide region using the accordion panel motion.',
 ['Click the trigger to toggle; control it with `open` + `on_open_change` or leave it uncontrolled with `default_open`.'],
 '''Collapsible::new("repos", Text::new("Starred repositories"), |_, _| {
    v_stack(6.).child("pelagornis/refineui").into_any_element()
})
.default_open(true);''')
page('calendar','Calendar','navigation','components/calendar.rs',['Calendar','Date'],'Month grid date picker with month navigation.',[],
 '''Calendar::new("due", Date::new(2026, 11, 1))
    .today(Date::new(2026, 11, 10))
    .selected(self.due)
    .on_select(cx.listener(|this, d: &Date, _, cx| {
        this.due = Some(*d);
        cx.notify();
    }));''')
page('menu','Menu','overlays','components/menu.rs',['Menu','MenuItem','DropdownMenu'],'Action list anchored to a trigger (web `Menu` / `MenuPopover`).',
 ['Menus close on selection, Escape or a click outside. ↑/↓ move, Enter activates.','Rows: `p sizeSmall`, radius large, gap `sizeXSmall`; titles `bodyMd`, shortcuts `bodyXs` tertiary; the panel is `min-w 200`, `p4`, radius 16 on the frosted surface.','`Menu::submenu(item, menu)` adds a hover-opened submenu (chevron row).','For right click at the pointer, use [Context Menu](/components/context-menu/).'],
 '''DropdownMenu::new(
    "file",
    Button::new("file-btn", "Open").variant(ButtonVariant::Secondary),
    Menu::new("file-menu")
        .label("File")
        .item(MenuItem::new("New").icon(IconName::Add).shortcut("⌘N").on_click(|_, _| {}))
        .divider()
        .item(MenuItem::new("Delete").icon(IconName::Delete).shortcut("⌘⌫")),
);''')
page('context-menu','Context Menu','overlays','components/context_menu.rs',['ContextMenu'],'Right-click menu that opens at the pointer, with hover submenus.',
 ['Opens at the pointer on right click and is kept 4px inside the window (web `placeInView`).','Submenus open on hover to the right of their row (flipping left near the edge) and close 150ms after the pointer leaves.','Escape, a selection or an outside press closes it.'],
 '''ContextMenu::new(
    "canvas",
    Button::new("rc", "Right click").variant(ButtonVariant::Secondary),
    Menu::new("ctx")
        .label("File")
        .item(MenuItem::new("New").icon(IconName::Add).shortcut("⌘N"))
        .divider()
        .submenu(
            MenuItem::new("More"),
            Menu::new("ctx-more").item(MenuItem::new("Rename")).item(MenuItem::new("Move")),
        ),
);''')
page('popover','Popover','overlays','components/popover.rs',['Popover','PopoverAlign'],'Anchored floating panel with rich content and a beak.',['The content closure receives a `close` callback.','Opens below the trigger, centered by default (`align_end()` / `.align(...)` for start / end); the 24×12 beak is part of the same outline as the panel.'],
 '''Popover::new("dims", Button::new("open", "Open").variant(ButtonVariant::Secondary), |close, _, _| {
    v_stack(16.)
        .child(Text::new("Dimensions"))
        .child(Button::new("done", "Done").on_click(move |_, _, cx| close(cx)))
        .into_any_element()
});''')
page('dialog','Dialog','overlays','components/dialog.rs',['Dialog','DialogSize'],'Modal dialog for focused tasks.',
 ['Keep the dialog rendered and drive `open(bool)` so the exit transition can play; clear your flag in `on_close` (scrim click, × or Escape).'],
 '''Dialog::new("discard", "Discard changes?")
    .description("This cannot be undone.")
    .size(DialogSize::Small)
    .footer(Button::new("ok", "Discard"))
    .open(self.confirm_open)
    .on_close(cx.listener(|this, _, _, cx| {
        this.confirm_open = false;
        cx.notify();
    }))''')
page('drawer','Drawer','overlays','components/dialog.rs',['Drawer','DrawerSize'],'Side panel that slides over content.',['Controlled like `Dialog`; open it from a regular `Button` (web `DrawerTrigger` is a default Button).','`footer_actions([...])` lays out the web footer states: one action full width, two split in halves, more as a trailing row.','No header close button by default (`close_button(true)` adds one); the scrim click and Escape close it.'],
 '''Drawer::new("details", "Drawer title")
    .size(DrawerSize::Small)
    .description("Supporting copy")
    .child(Text::new("Drawer content"))
    .footer_actions([
        Button::new("cancel", "Cancel").variant(ButtonVariant::Outline).full_width(true),
        Button::new("submit", "Submit").full_width(true),
    ])
    .open(self.open)
    .on_close(|_, _| {});''')
page('table','Table','data','components/table.rs',['Table','Column','Align'],'Sortable, selectable data table.',['Columns are flexible by default; use `width(px)` or `fraction(0..1)` to size them.'],
 '''Table::new("files", [
    Column::new("File").fraction(0.5).sorted(true),
    Column::new("Lines").width(80.).align(Align::End),
])
.text_rows([["button.rs", "412"], ["input.rs", "890"]])
.striped(true);''')
page('scroll-area','Scroll Area','data','components/scroll_area.rs',['ScrollArea','ScrollEdgeItem'],'Scrollable region with an overlay scrollbar and edge blur.',
 ['The 10px rail floats over the content (inset 2px) and shows while hovered, dragging, or for a moment after scrolling.','Edge blur: rows wrapped in `scroll_edge_item` (web `data-scroll-edge-item`) soften as they cross the outer 25% of the viewport on an edge that hides more content; each edge ramps in over the first 25% of scroll distance. GPUI has no blur filter, so a row fades instead of blurring. Turn it off with `edge_blur(false)`.'],
 '''div().h(px(256.)).child(
    ScrollArea::new("tags").children(tags.iter().map(|t| {
        scroll_edge_item(
            t.clone(),
            div().px(px(16.)).py(px(10.)).border_b_1().child(Text::new(t.clone())),
        )
    })),
)''')
page('resizable','Resizable','data','components/resizable.rs',['ResizablePanelGroup','ResizablePanel','ResizableHandle','ResizableOrientation'],'Panel group with draggable separators.',
 ['Sizes are percentages (`default_size` 50, `min_size` 10, `max_size` 100); a drag moves the boundary between the two neighbours within both limits.','`with_handle(true)` draws the 4×32 grip; the 12px hit area has no rule. Arrow keys move 2% (Shift 10%), Home / End jump to the limits.','`size(pct)` controls a panel; `on_layout` reports sizes when a drag or key resize settles.'],
 '''ResizablePanelGroup::new("split")
    .panel(ResizablePanel::new().default_size(35.).min_size(20.).child(sidebar))
    .handle(ResizableHandle::new().with_handle(true))
    .panel(ResizablePanel::new().default_size(65.).min_size(30.).child(main))''')
page('colors','Colors','foundations','theme_colors.rs',['ThemeColors'],'Semantic color aliases from RefineUI, resolved for the current appearance.',
 ['Read colors with `cx.theme().colors`. Never hard-code hex values in components.','Regenerate from the web tokens with `python3 scripts/sync_tokens.py`.'],
 '''let c = cx.theme().colors;
div().bg(c.background_surface).border_1().border_color(c.border_subtle).text_color(c.foreground_secondary)''')
page('typography','Typography','foundations','components/text.rs',['TextVariant'],'The RefineUI type scale (Inter, with Menlo for code).',[],
 '''Text::new("Section title").variant(TextVariant::SubTitle1);''')
page('elevation','Elevation','foundations','theme.rs',['Theme'],'Shadow (key + ambient) and radius tokens.',['Use `cx.theme().shadow(Elevation::FLOATING)` or the `StyledExt::elevation` helper.'],
 '''div().rounded(px(12.)).shadow(cx.theme().shadow(Elevation::RAISED))''')
page('icon','Icon','foundations','icon.rs',['Icon','IconSize','IconStyle'],'RefineUI System Icons in regular and filled styles.',['Icons are embedded SVGs served by `refineui::Assets`; register it with `Application::new().with_assets(refineui::Assets)`.','Add icons with `python3 scripts/gen_icons.py <name>…`.'],
 '''Icon::new(IconName::Settings).size(IconSize::Large).color(cx.theme().colors.foreground_secondary);
Icon::new(IconName::Star).style(IconStyle::Filled);''')

# ---- added with RefineUI web 0.5.0 ----------------------------------------
page('button-group','Button Group','actions','components/button_group.rs',['ButtonGroup'],'Related buttons joined in one tube (attached) or spaced apart (web `ButtonGroup`).',
 ['Buttons inherit the group `size` and `variant` unless they set their own.','Attached groups sit in a 2px-padded `backgroundBrandSubtle` tube with a thin `borderDefault` edge; its radius is the button radius + 2 so the corners stay concentric.','`orientation(Axis::Vertical)` stacks the buttons.'],
 """ButtonGroup::new()
    .buttons([
        Button::new("left", "Left"),
        Button::new("center", "Center"),
        Button::new("right", "Right"),
    ])
    .size(ControlSize::Medium);""")
page('toolbar','Toolbar','actions','components/toolbar.rs',['Toolbar','ToolbarGroup'],'A row of ghost buttons and toggles with groups and dividers (web `Toolbar`).',
 ['Buttons default to `Ghost` and inherit the toolbar size (default `Small`).','`attached(true)` (default) draws the floating bar: hairline `borderSubtle`, `backgroundSurface`, 4px padding, raised shadow.','← / → move focus between the toolbar items.'],
 """Toolbar::new("format")
    .label("Formatting")
    .group(ToolbarGroup::new()
        .toggle(ToggleButton::icon_only("bold", IconName::Edit, self.bold)))
    .divider()
    .button(Button::icon_only("copy", IconName::Copy).tooltip("Copy"));""")
page('date-input','Date Input','forms','components/date_input.rs',['DateInput'],'Field that opens a Calendar popover — one date or a range (web `DateInput`).',
 ['Values are `Date`; the field shows them like `Intl` short dates ("Nov 10, 2026").','`DateInput::range` shows two months and closes after the end date is picked.','Default size is `Large` (48px), like the web field.'],
 """DateInput::new("due", self.due)
    .on_change(cx.listener(|this, d: &Date, _, cx| {
        this.due = Some(*d);
        cx.notify();
    }));""")
page('time-input','Time Input','forms','components/time_input.rs',['TimeInput','Time'],'Field that opens hour / minute wheels (web `TimeInput`).',
 ['Wheels are 140px tall with 32px rows; rows fade toward the edges and the wheel snaps to a row 120ms after scrolling stops.','`minute_step(15)` and `twelve_hour(true)` match the web props.'],
 """TimeInput::new("start", Some(Time::new(9, 30)))
    .minute_step(15)
    .on_change(cx.listener(|this, t: &Time, _, cx| {
        this.start = Some(*t);
        cx.notify();
    }));""")
page('label','Label','forms','components/text.rs',['Label'],'Form label in three sizes with a required marker (web `Label`).',['Use inside `Field` or next to any control.'],
 """Label::new("Email").required(true);""")
page('tag','Tag','display','components/chip.rs',['Chip','ChipVariant'],'Tag — an alias of Chip, exactly like the web (`Tag` re-exports `Chip`).',['`Tag` / `TagVariant` are type aliases of `Chip` / `ChipVariant`; `Chip::tag` is the same as `Chip::new`.'],
 """Tag::tag("rust", "Rust").variant(TagVariant::Outline);""")
page('result','Result','display','components/result.rs',['ResultView','ResultVariant'],'Outcome screen with an icon, title, description and actions (web `Result`).',
 ['Named `ResultView` so it does not shadow `std::result::Result` in glob imports.','The variant picks the status icon and color; `figure` replaces the icon with any element.'],
 """ResultView::new(ResultVariant::Success)
    .title("Payment received")
    .description("Your receipt was sent to your email.")
    .action(Button::new("done", "Done").variant(ButtonVariant::Primary));""")
page('message','Message','display','components/message.rs',['Message'],'Conversation row: avatar, bubbles and a footer (web `Message`).',['Pair with `BubbleGroup` for runs of bubbles from one sender.'],
 """Message::new()
    .avatar(Avatar::new("Ada Lovelace"))
    .child(BubbleGroup::new([Bubble::new("Hi!"), Bubble::new("Got a minute?")]))
    .footer("Ada · 9:41");""")
page('bubble','Bubble','display','components/bubble.rs',['Bubble','BubbleVariant','BubbleAlign','BubbleReactionsSide','BubbleGroup'],'Chat bubble in seven variants with grouped corners and reactions (web `Bubble`).',
 ['`BubbleGroup` flattens the sender-side corners of consecutive bubbles (16px → 8px), like the web group.','Reactions sit in a pill that overlaps the bubble edge by 75% of its height.','A bubble fills its row (`w_full`) and aligns its content start or end.'],
 """BubbleGroup::new([
    Bubble::new("Shipped in 0.5.0").variant(BubbleVariant::Tinted).align(BubbleAlign::End),
    Bubble::new("Docs are up").align(BubbleAlign::End).reactions(["🎉"]),
]);""")
page('marker','Marker','display','components/marker.rs',['Marker','MarkerVariant'],'Timeline marker — inline or as a divider — with an optional live pulse (web `Marker`).',[],
 """Marker::new("today", "Today").variant(MarkerVariant::Divider);
Marker::new("live", "Live").status(true);""")
page('code-block','Code Block','display','components/code_block.rs',['CodeBlock'],'Syntax-highlighted code with title, line numbers, highlighted lines and a copy button (web `CodeBlock`).',
 ['`.language("rust")` colours tokens with the Night Owl Light / Night Owl themes the web docs use through Shiki (light / dark follow the theme). Highlighting runs on `syntect` (default feature `syntax-highlight`) and is cached per code string.','Languages: rust, ts / tsx / js, sh, json, css, html, python, yaml, markdown, c / c++, go, java, sql, diff. Unknown languages (or the feature turned off) fall back to plain text.','The copy button writes to the clipboard and shows a check for 1.6s.'],
 """CodeBlock::new("snippet", "fn main() {\\n    println!(\\"hi\\");\\n}")
    .title("main.rs")
    .language("rust")
    .line_numbers(true);""")
page('chart','Chart','display','components/chart.rs',['Chart','ChartKind','ChartSeries'],'Bar and monotone line charts with axis, grid, hover cursor and tooltip (web `Chart`).',
 ['Geometry follows the web 800 × 320 viewBox (aspect 5:2): bars fill half their band with a 12px top radius; the line is a monotone cubic with a fading area.','The scale max rounds up to 1 / 2 / 5 / 10 × 10ⁿ (`nice_max`), with 5 grid lines by default.'],
 """Chart::bars("revenue", [186., 305., 237., 273., 209., 214.],
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]);
Chart::line("visitors", values, labels).series(ChartSeries::Info);""")
page('carousel','Carousel','display','components/carousel.rs',['Carousel'],'Looping slides with outline buttons and a sliding-pill meter (web `Carousel`).',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','Slides are built on demand by index, so the loop continues in the same direction past the last slide.','Drag the stage to swipe: the track follows the pointer and a release past 18% of the width (web `DRAG_COMMIT_RATIO`) moves one slide; shorter drags spring back. Buttons, the meter and ← / → (while focused) also navigate.'],
 """Carousel::new("hero", 4, |ix, _window, _cx| {
    div().size_full().child(format!("Slide {}", ix + 1)).into_any_element()
})
.height(220.);""")
page('navigation-menu','Navigation Menu','navigation','components/navigation_menu.rs',['NavigationMenu','NavigationLink'],'Top navigation with links and dropdown panels (web `NavigationMenu`).',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','While a panel is open, hovering another trigger switches to it.','`menu` builds the compact frosted dropdown; `panel(label, layout, content)` takes any content in a `NavigationLayout` — `Dropdown`, `Floating` (mega panel, see Navigation Menu · Mega) or `Overlay` (full-width sheet, see Navigation Menu · Overlay).','`default_open(index)` opens an entry on first render (previews).'],
 """NavigationMenu::new("nav")
    .link("Overview", true, |_, _| {})
    .menu("Products", [
        NavigationLink::new("Design system").description("Tokens and components"),
        NavigationLink::new("Icons"),
    ]);""")
page('navigation-menu-mega','Navigation Menu · Mega','navigation','components/navigation_menu.rs',['NavigationLayout'],'Floating mega panel centred under the bar (web `NavigationMenuContent layout="floating"`).',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','Centred in the window (clamped 8px from the edges), 4px below the trigger, at most 1000px wide (`navigationMenuMegaWidth`).','`backgroundPrimary` fill, hairline `borderDefault`, `roundedXXLarge`, raised shadow.'],
 """NavigationMenu::new("nav")
    .panel("Products", NavigationLayout::Floating, |_, cx| mega_columns(cx).into_any_element());""")
page('navigation-menu-overlay','Navigation Menu · Overlay','navigation','components/navigation_menu.rs',['NavigationLayout'],'Full-width sheet from the bar edge over a scrim (web `NavigationMenuContent layout="overlay"`).',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','Spans the window from the bar\'s bottom edge; the rest of the window is covered by the `surfaceOverlay` scrim.','Hovering or clicking the scrim closes the sheet (web pointer-enter close).','No rounding; hairline bottom border; content centred.'],
 """NavigationMenu::new("nav")
    .panel("Products", NavigationLayout::Overlay, |_, cx| mega_columns(cx).into_any_element());""")
page('footer','Footer','navigation','components/footer.rs',['Footer','FooterGroup'],'Sitemap groups and a social | brand | locale meta row (web `Footer`).',[],
 """Footer::new("footer")
    .group(FooterGroup::new("Product", ["Components", "Tokens"]))
    .brand(None, "RefineUI", Some("© 2026 Pelagornis".into()))
    .social([IconName::Mail, IconName::Link]);""")
page('command','Command','overlays','components/command.rs',['Command','CommandGroup','CommandItem','CommandEvent'],'Searchable command list with groups, shortcuts and keyboard navigation (web `Command`).',
 ['**Optional** — behind the `web-extras` cargo feature (web-oriented, not part of Desktop Core): `refineui = { version = "0.1", features = ["web-extras"] }`.','`Command` is an entity; subscribe to `CommandEvent::Select`.','Typing filters items by every word of the label, value or keywords; ↑ / ↓ move the indicator and Enter selects.','Put it in a `Dialog` for a ⌘K palette.'],
 """let palette = cx.new(|cx| Command::new(window, cx)
    .group(CommandGroup::new("Suggestions")
        .item(CommandItem::new("Calendar").icon(IconName::Calendar))
        .item(CommandItem::new("Search files").shortcut("⌘P"))));
cx.subscribe(&palette, |this, _, CommandEvent::Select(value), cx| {
    this.run(value, cx);
})
.detach();""")
page('dropdown','Dropdown','overlays','components/menu.rs',['DropdownMenu'],'A trigger that opens a Menu below it (web `Dropdown`). Optional real backdrop blur on macOS.',
 ['`align_end()` aligns the menu with the trigger end edge.','`refineui::set_frost_mode(FrostMode::Native, cx)` makes dropdowns open in an OS-blurred pop-up window on macOS (prototype; see the Blur guide). Other platforms keep the opaque panel.'],
 """DropdownMenu::new(
    "file",
    Button::new("file-btn", "File").icon_end(IconName::ChevronDown),
    Menu::new("file-menu").item(MenuItem::new("New")).divider().item(MenuItem::new("Open…")),
);""")
page('layout','Layout','foundations','components/layout.rs',[],'Stack, Grid, Container, Box and Spacer presets (web `Layout`).',
 ['`container()` / `container_with()` (centred page width) are behind the optional `web-extras` feature; desktop layouts use split views and panels instead.','`h_stack(gap)` / `v_stack(gap)` — web `Stack`; `spacer()` — web `Spacer`.','`grid(columns, gap)` — web `Grid`; give children `.col_span(n)` for `GridItem span`.','`container()` — web `Container` (24px inline padding); `token_box(...)` — web `Box`.','In a grid, avoid children that are `w_full()` flex rows (Taffy resolves their percentage width against the whole grid).'],
 """grid(3, 8.)
    .child(card_a)
    .child(card_b.col_span(2));
container().child(h_stack(8.).child(title).child(spacer()).child(actions));""")


# ---- Desktop shell (Phase 2) ----------------------------------------------
page('application-shell','Application Shell','desktop','shell/app_shell.rs',['ApplicationShell','ShellDock','shell/mod.rs:ShellSpec','shell/mod.rs:ToolbarPlacement','shell/mod.rs:RailEdge'],
 'The window-level layout: titlebar (with a unified toolbar), menu bar, activity bar, docks, workspace and status bar, laid out per OS.',
 ['Compose the parts in one builder; every slot is optional. Per-OS metrics (titlebar height, traffic-light reserve, menu bar) come from the `PlatformAdapter` and the current `Density`.',
  'On macOS the menus go to the system menu bar (call `set_system_menus` once); on Windows / Linux the same `AppMenu` model renders as an in-window `MenuBar`.',
  '**F6 / Shift-F6** cycle focus between regions (menu bar → toolbar → activity bar → docks → workspace → status bar); Tab then moves inside the region. **F10** (and Alt on Windows / Linux) focuses the menu bar. A region gets an inset `borderFocus` outline only for keyboard focus.',
  'On Linux the shell wraps itself in `WindowFrame` (client-side decorations: rounded corners, shadow, resize edges). Use `.embedded(true)` and `.os(..)` for previews inside another window.',
  'Run the full demo: `cargo run -p refineui-gallery --bin refineui-shell`.'],
 """ApplicationShell::new("app")
    .title("refineui-gpui")
    .subtitle("main.rs")
    .menus(menus)                       // Vec<AppMenu>
    .toolbar(Toolbar::new("tb").fill(true).button(sidebar).spacer().button(run))
    .activity_bar(ActivityBar::new("activity").items(items).selected("explorer"))
    .left_dock(ShellDock::new(explorer, 248.))
    .center(editor)
    .bottom_dock(ShellDock::new(terminal, 180.))
    .status_bar(StatusBar::new("status").leading(StatusItem::text("branch", "main")))
    .overlay(self.palette.clone());""")
page('titlebar','Titlebar','desktop','shell/titlebar.rs',['Titlebar','WindowFrame'],
 'The window titlebar: draggable, double-click to zoom, with title / subtitle, leading and trailing slots or a unified toolbar.',
 ['**macOS** — transparent titlebar (`appears_transparent`), AppKit traffic lights inset to the titlebar centre (`traffic_light_position`), 52px unified / 38px plain; the leading reserve disappears in full screen.',
  '**Windows** — caption buttons drawn by RefineUI (46 × titlebar height) tagged with `WindowControlArea::{Min, Max, Close}` so the system hit test handles them (snap layouts on maximize hover). The rest of the bar is `WindowControlArea::Drag`.',
  '**Linux** — client-side decorations: round header-bar buttons, `start_window_move` on press, right-click opens the window menu, `WindowFrame` draws the rounded frame, shadow and resize edges.',
  'Double-click follows the platform: `titlebar_double_click` on macOS (respects the user preference), maximize toggle elsewhere. Interactive children are occluded so they never start a window drag.',
  'Window options come from `window_options(&WindowSpec, cx)`, which sets the transparent titlebar, traffic-light point, min size and decorations to match.'],
 """let spec = WindowSpec::new(WindowPreset::Main, "My App").restore(WindowState::load(&path));
cx.open_window(window_options(&spec, cx), |window, cx| { /* … */ })?;

Titlebar::new("titlebar")
    .title("refineui-gpui")
    .subtitle("main.rs")
    .trailing(Button::icon_only("share", IconName::Share).variant(ButtonVariant::Ghost));""")
page('window-controls','Window Controls','desktop','shell/titlebar.rs',['WindowControls','ControlButtons','shell/window.rs:WindowPreset','shell/window.rs:WindowState','shell/window.rs:WindowSpec'],
 'Minimize, maximize / restore and close buttons drawn per OS, plus window presets and size / position restore.',
 ['macOS draws nothing (AppKit owns the traffic lights) except in previews. Windows: Segoe-style glyphs, close turns `#C42B1C` on hover. Linux: 24px round buttons.',
  'Buttons the platform reports as unsupported (`window.window_controls()`) or the preset disables (`WindowPreset::Settings` has no minimize / maximize) are omitted.',
  '**Presets** — `Main` (1280 × 800, min 720 × 480), `Document`, `Settings` (fixed-ish, no maximize) and `Utility`.',
  '**Restore** — `track_window_state(window, cx, path)` saves the bounds, maximized and full-screen state on every move / resize and on close; `WindowState::load` + `sanitized(displays, min)` drop placements whose titlebar would be off every connected display (unplugged monitor) and clamp the size to the preset minimum.'],
 """WindowControls::new("controls").buttons(WindowPreset::Settings);

// Remember where the window was.
let _sub = track_window_state(window, cx, config_dir.join("window.state"));""")
page('menu-bar','Menu Bar','desktop','shell/menu_bar.rs',['MenuBar','AppMenu','AppMenuItem'],
 'One `AppMenu` model for the system menu bar on macOS and an in-window menu bar on Windows and Linux.',
 ['Items name GPUI actions (`"editor::Save"`); shortcuts are read from the installed keymap, so the menu, tooltips and the command palette always agree.',
  '`set_system_menus(&menus, cx)` builds `gpui::Menu`s with key equivalents and standard `OsAction`s (Undo, Redo, Cut, Copy, Paste, Select All) and returns any unregistered action names.',
  'In-window: click a title or press **F10** / **Alt**; ← / → move between menus, ↓ / Enter open, Escape closes and returns focus. While one menu is open, hovering another title switches to it.'],
 """let menus = vec![
    AppMenu::new("File").action("New File", "demo::NewFile").separator().action("Save", "demo::Save"),
    AppMenu::new("Edit").action("Undo", "demo::Undo").action("Copy", "demo::Copy"),
];
if Os::current() == Os::MacOs {
    set_system_menus(&menus, cx);
}
MenuBar::new("menubar", menus);""")
page('status-bar','Status Bar','desktop','shell/status_bar.rs',['StatusBar','StatusItem','StatusTone'],
 'The strip at the bottom of the window: leading and trailing groups of text, buttons, progress and custom items.',
 ['Height follows density (`status_bar_height`: 22 / 24 / 28); items use `captionMd` text and 12px icons.',
  'Buttons are quiet (fill on hover only) and can carry a tooltip with a keymap shortcut: `.tooltip("Toggle Panel", Some("workspace::TogglePanel"))`.',
  '`accent(true)` paints the bar `backgroundBrand` (e.g. while debugging or connected to a remote).'],
 """StatusBar::new("status")
    .leading(StatusItem::button("branch", "main", |_, _| {}).icon(IconName::Git))
    .leading(StatusItem::text("problems", "2 warnings").icon(IconName::Warning).tone(StatusTone::Warning))
    .leading(StatusItem::progress("index", "Indexing", 0.6))
    .trailing(StatusItem::text("pos", "Ln 12, Col 4"));""")
page('activity-bar','Activity Bar','desktop','shell/activity_bar.rs',['ActivityBar','ActivityItem','ActivityBarStyle','ActivityBarPreset','ItemShape','IndicatorStyle','LabelMode','BarSide','ActivityItemState','ItemBadge'],
 'A vertical rail of views (Explorer, Search, Source Control …) — RefineUI style by default, fully restylable: shape, size, indicator, colours, labels, custom faces.',
 ['**Refine** (default) — density-sized bar (48px comfortable), 36px rounded tiles, the selected item sits on a `backgroundSurfaceSelected` plate; tooltips name each item.',
  '**Navigation rail** — 80px, captions under the icons (`LabelMode::Below`). **Expanded** — `expanded(true)` shows labels beside the icons at `expanded_width`.',
  '**Rail preset** — `ActivityBar::rail` keeps the former Side Dock look (circle → squircle morph, edge pill, brand fill) for chat-style apps.',
  'Every knob is on `ActivityBarStyle`: `shape` (Rounded / Square / Circle / Morph), `indicator` (None / Bar / Dot / Plate), `item_size`, `gap`, `icon`, `border` and every colour. `render_item` draws the face yourself; the bar keeps hit-testing, focus, indicator and tooltip.',
  'Items: `icon`, `image`, `initials` (+`tint`), `folder`; `badge(n)`, `dot()`, `unread`, `disabled`, `action` (shows the keymap shortcut in the tooltip). `header` / `footer` slots and `footer_item`s pin to the ends.',
  'Keyboard: the bar is one tab stop; ↑ / ↓ move, Enter / Space activate, **⌥↑ / ⌥↓** reorder when `on_reorder` is set (drag also reorders, with a drop indicator).'],
 """ActivityBar::new("activity")
    .items([
        ActivityItem::icon("explorer", "Explorer", IconName::Document),
        ActivityItem::icon("search", "Search", IconName::Search).action("workspace::Find"),
        ActivityItem::icon("git", "Source Control", IconName::Git).badge(3),
    ])
    .footer_item(ActivityItem::icon("settings", "Settings", IconName::Settings))
    .indicator(IndicatorStyle::Bar)
    .shape(ItemShape::Rounded(8.))
    .selected(self.view.clone())
    .on_select(cx.listener(|this, id: &SharedString, _, cx| { this.view = id.clone(); cx.notify(); }))
    .on_reorder(move |from, to, _window, cx| {
        // `to` is the final index: items.remove(from); items.insert(to, item)
    });""")
page('shortcut-hint','Shortcut Hint','desktop','components/shortcut_hint.rs',['ShortcutHint','ShortcutStyle'],
 'A key combination formatted for the current OS — ⇧⌘P on macOS, Ctrl+Shift+P on Windows and Linux.',
 ['`ShortcutHint::keys("secondary-shift-p")` formats literal keys (`secondary` = ⌘ on macOS, Ctrl elsewhere).',
  '`ShortcutHint::action("editor::Save")` looks the binding up in the installed keymap (`KeybindingManager::install`) and renders nothing when the action is unbound.',
  '`ShortcutStyle::Keycaps` draws one `Kbd` cap per key; `Text` is the compact inline form used in menus. Tooltips (`Button::tooltip_action`) and menu items show the same label.'],
 """ShortcutHint::keys("secondary-shift-p");
ShortcutHint::action("editor::Save").style(ShortcutStyle::Text);
Button::icon_only("save", IconName::Save).tooltip("Save").tooltip_action("editor::Save");""")
page('drag-and-drop','Drag and Drop','desktop','components/drag.rs',['DragHandle','DragPreview','DropIndicator','DropZone','interaction/dnd.rs:DockZone','interaction/dnd.rs:DropEdge'],
 'The visual parts of drag and drop: a grip, the chip that follows the pointer, the insertion line and docking drop zones.',
 ['GPUI owns the gesture (`on_drag`, `on_drag_move`, `on_drop`); these are the visuals, and `interaction::dnd` is the geometry (`drop_edge`, `insertion_index`, `reorder`, `dock_zone`).',
  '`DragHandle` — 2 × 3 dot grip, `grab` cursor; keyboard reordering is ⌥↑ / ⌥↓ on the focused row instead of a handle tab stop.',
  '`DragPreview` — frosted popover chip with an icon and a count badge for multi-item drags; return it from `on_drag`.',
  '`DropIndicator` — 2px `borderFocus` line with a ring cap between rows or tabs. `DropZone` — `backgroundBrandSubtle` overlay on the half / whole pane a docking drop would occupy.'],
 """div()
    .child(DragHandle::new("grip"))
    .on_drag(RowDrag { from: ix }, |d, _, _, cx| cx.new(|_| DragPreview::new("Keymap").icon(IconName::Document)))
    .on_drop(|d: &RowDrag, _, cx| { /* reorder(&mut items, d.from, to) */ });

// In a pane:
div().relative().child(content).child(DropZone::new(self.hover_zone));""")
page('selection-box','Selection Box','desktop','components/selection_box.rs',['SelectionArea','SelectionLayout'],
 'Rubber-band (marquee) selection over a grid or list, with click, ⌘/Ctrl-click, Shift-click and keyboard selection.',
 ['Press on empty space and drag: items touching the marquee are selected live; Shift or ⌘/Ctrl adds to the previous selection. A press under 4px clears it.',
  'Click selects, ⌘/Ctrl-click toggles, Shift-click extends from the anchor. ⌘/Ctrl-A selects all, Escape clears, arrows move (Shift extends).',
  'Selected items sit on a `backgroundBrandSubtle` plate with a `borderFocus` hairline; the marquee is `backgroundBrandSubtle` at 60% with a hairline outline.'],
 """SelectionArea::new("files")
    .item("a.png", tile("a.png"))
    .item("b.png", tile("b.png"))
    .selected(["a.png"])
    .on_change(|keys, _, _| println!("{} selected", keys.len()));""")
page('command-palette','Command Palette','desktop','components/command_palette.rs',['CommandPalette','PaletteCommand'],
 'The ⇧⌘P / Ctrl+Shift+P action search: grouped commands with icons and keymap shortcuts, built on `Command`.',
 ['`CommandPalette` is an entity: render it as an overlay of the window root and toggle it from a `ToggleCommandPalette` handler.',
  'Commands name GPUI actions; running one closes the palette, restores focus to where it was and dispatches the action there.',
  'Shortcut labels are refreshed from the installed keymap every time it opens. Typing filters by label and keywords; ↑ / ↓ move, Enter runs, Escape closes.'],
 """let palette = cx.new(|cx| CommandPalette::new(vec![
    PaletteCommand::new("Save", "editor::Save").group("File").icon(IconName::Save),
    PaletteCommand::new("Toggle Sidebar", "workspace::ToggleSidebar").group("View"),
], window, cx));

root(cx)
    .on_action(cx.listener(|this, _: &ToggleCommandPalette, window, cx| {
        this.palette.update(cx, |p, cx| p.toggle(window, cx));
    }))
    .child(self.palette.clone());""")

# Extra MDX after the usage section.
EXTRA={
 'application-shell':"""## The shell demo

A complete desktop app built only from these parts — titlebar with a unified toolbar, menu bar (the system menu bar on macOS), activity bar, explorer sidebar, editor placeholder with tabs, bottom panel, status bar and the command palette. Window placement is restored on the next launch.

<Screenshot id="shell-demo" alt="The refineui-shell demo window" />

```sh
cargo run -p refineui-gallery --bin refineui-shell
```

Or press **Shell demo** in the gallery's top bar.

""",
}

MOTION={
 'button':'Pressing scales the button to 0.96 over `duration-fast` (`motion-scale-press`); background, border and text colors tween over `duration-fast` (ease-out); the focus ring appears on keyboard focus only.',
 'toggle-button':'Pressed / hover plates tween over `duration-fast`.',
 'segmented-control':'The thumb slides over `duration-medium` while its width follows over `duration-slow` (both emphasized), so the trailing edge stretches when it travels.',
 'link':'Color and underline change over `duration-fast`.',
 'input':'Border and focus halo tween over `duration-fast`.',
 'textarea':'Border and focus halo tween over `duration-fast`.',
 'search-field':'Plate and border tween over `duration-fast` (emphasized).',
 'input-otp':'Empty cells pulse like Skeleton, staggered by `index × duration-fast`; plates and borders tween over `duration-fast` (emphasized). Respects reduced motion.',
 'checkbox':'Fill and check mark fade in over `duration-fast`.',
 'radio':'The inner dot grows over `duration-fast`.',
 'switch':'Thumb slides and the track recolors over `duration-medium`.',
 'slider':'Thumb grows on hover and recolors over `duration-fast`.',
 'select':'The popup fades in with the frosted-surface motion and a hover plate slides between options, item-aligned so the selected row sits over the trigger; trigger and items scale to 0.98 when pressed.',
 'badge':'Interactive badges tween their plate over `duration-fast`.',
 'chip':'Plate and remove-glyph opacity tween over `duration-fast`.',
 'card':'Hover / pressed plates and the selection ring tween over `duration-fast`.',
 'spinner':'A brand arc rotates every 800ms on a `brandSubtle` ring.',
 'progress':'The fill grows from 0 over `duration-progress`; the indeterminate bar sweeps with ease-in-out.',
 'progress-stepper':'The fill width moves with `duration-panel` (emphasized); pips and labels recolor over `duration-fast`, and upcoming steps preview `foregroundSecondary` on hover.',
 'stepper':'Indicators recolor over `duration-medium`; each connector is one rail between neighbouring markers that fills left → right over `duration-panel` (emphasized).',
 'skeleton':'Opacity pulses 1 → 0.5 → 1 every 1.5s; static under reduced motion.',
 'tooltip':'Fades in and slides 6px away from the pointer over `duration-fast` (emphasized); the beak is part of the bubble outline.',
 'toast':'Enters from `translateY(-100%)` over 280ms (emphasized), leaves over 200ms (standard); the stack fans out over `duration-panel` on hover.',
 'tabs':'The underline indicator slides and resizes between tabs.',
 'breadcrumb':'Link plates tween over `duration-fast`.',
 'pagination':'Border width and plate tween over `duration-fast`.',
 'sidebar':'A hover plate slides between links over `duration-medium` (emphasized); link color and the active plate tween over `duration-fast`; collapsing animates the width over `duration-panel` (panel curve) with the rail frozen; the peek panel slides in over `duration-overlay` (standard).',
 'tree':'A hover plate slides between rows over `duration-medium` (emphasized). Chevron rotates 90° over `duration-medium` (emphasized); children expand with the accordion panel motion.',
 'accordion':'Height animates over 380ms with the panel curve while content fades and slides 6px; the chevron rotates 180°.',
 'collapsible':'Same height + content motion as Accordion.',
 'calendar':'Day plates tween over `duration-normal` (emphasized) and press to 0.96; range mode shows two months with an endpoint band.',
 'menu':'Panel fades in (frosted motion); one hover plate slides between rows over `duration-medium` (emphasized) — fading in where it first lands, out in place; rows scale to 0.98 when pressed; submenus open on hover.',
 'popover':'Fades in and slides 6px away from the trigger over `duration-fast` (emphasized), like the web `useFloatingPresence` (the web 0.96 scale is not reproducible in GPUI).',
 'dialog':'Scrim fades over 280ms and the panel fades and rises 8px over `duration-panel`; both reverse on close.',
 'drawer':'Scrim fades over `duration-overlay` and the panel slides in from the edge over `duration-panel` (panel curve); reverses on close.',
 'table':'A single `surfaceHover` plate slides between hovered rows over `duration-medium` (emphasized); selection tweens over `duration-fast`.',
 'scroll-area':'The overlay scrollbar fades in and out over `duration-fast`; edge blur follows the scroll position.',
 'context-menu':'Panel fades in at the pointer (frosted motion); submenus open on hover and close 150ms after leaving.',
 'button-group':'Buttons keep their own press (0.96) and color motion.',
 'toolbar':'Buttons and toggles keep their own press and plate motion.',
 'date-input':'The popover fades and slides 6px down over `duration-fast` (emphasized); the border and focus halo tween over `duration-fast`.',
 'time-input':'Same popover motion as Date Input; wheels snap to a row 120ms after scrolling stops and rows fade toward the edges.',
 'marker':'The status dot pulses like Skeleton (1.5s).',
 'code-block':'The copy glyph swaps to a check for 1.6s.',
 'navigation-menu-mega':'The panel fades in and rises 6px over `duration-normal` (emphasized); closes over `duration-fast`.',
 'navigation-menu-overlay':'The sheet grows to its content height while the scrim fades in, both over `duration-panel` (panel curve); reverses on close.',
 'activity-bar':'Item fill, ink and the indicator tween over `duration-fast`; `ItemShape::Morph` tiles round → squircle over `duration-fast`; the bar indicator grows over `duration-medium` (emphasized); tooltips fade and slide 6px over `duration-fast`.',
 'command-palette':'Opens and closes instantly (desktop palettes do not animate in); the result indicator moves over `duration-fast`.',
 'menu-bar':'Menus open instantly; hover-switching between open menus is immediate, like native menu bars.',
 'chart':'Bars grow from the baseline and the line draws in over `duration-panel` (emphasized); the hover cursor and tooltip fade over `duration-fast`.',
 'carousel':'Slides glide over `duration-panel` and the meter thumb over `duration-medium` (both emphasized). While dragging the track follows the pointer 1:1; on release it settles over `duration-panel` (emphasized).',
 'navigation-menu':'Panels fade and slide in over `duration-normal` (emphasized); the chevron turns 180° over `duration-fast`.',
 'command':'A `surfaceHover` indicator slides between items over `duration-medium` (emphasized); rows press to 0.96; the empty state fades.',
 'dropdown':'Same as Menu. With native frost the panel is a separate OS window whose alpha fades in over `duration-normal`; it closes immediately.',
 'resizable':'The grip recolors `borderDefault` → `borderStrong` on hover, focus and drag over `duration-fast`.',
}
GROUPS={'foundations':'Foundations','actions':'Actions','forms':'Forms','display':'Display','navigation':'Navigation','overlays':'Overlays','data':'Data','desktop':'Desktop shell'}
os.makedirs(OUT+'components',exist_ok=True); os.makedirs(OUT+'foundations',exist_ok=True); os.makedirs(OUT+'shell',exist_ok=True)
# story order from gallery
order=re.findall(r'id: "([a-z-]+)"', ''.join(open(ROOT+'/examples/gallery/src/stories/'+f).read() for f in ['foundations.rs','actions.rs','forms.rs','display.rs','navigation.rs','overlays.rs','data.rs','desktop.rs','extra.rs']))
assert set(order)==set(P), set(order)^set(P)
for n,id in enumerate(order):
    title,group,file,types,desc,notes,example=P[id]
    folder='foundations' if group=='foundations' else 'shell' if group=='desktop' else 'components'
    notes_md='\n'.join(f'- {x}' for x in notes)
    motion_md=f"## Motion\n\n{MOTION[id]}\n\n" if id in MOTION else ''

    body=f'''---
title: {title}
description: {json.dumps(desc, ensure_ascii=False)}
sidebar:
  order: {n}
---

import Screenshot from '../../../components/Screenshot.astro';

{desc}

<Screenshot id="{id}" alt="{title} variants and states" />

## Usage

{notes_md}

```rust
use refineui::prelude::*;

{example}
```

Open the live, interactive version in the gallery:

```sh
cargo run -p refineui-gallery -- --story {id}
```

{EXTRA.get(id, '')}{motion_md}## API

Source: [`crates/refineui/src/{file}`](https://github.com/pelagornis/refineui-gpui/blob/main/crates/refineui/src/{file})

{api_md(file,types)}
'''
    body=body.replace('## Usage\n\n\n','## Usage\n\n')
    open(f'{OUT}{folder}/{id}.mdx','w').write(body)
stale=[]
for d in ('components','shell'):
    for f in os.listdir(OUT+d):
        if f.endswith('.mdx') and (f[:-4] not in P or (P[f[:-4]][1]=='desktop')!=(d=='shell')):
            os.remove(OUT+d+'/'+f); stale.append(d+'/'+f)
print(len(order),'pages', 'removed', stale)
