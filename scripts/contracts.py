#!/usr/bin/env python3
"""Component contracts helper (crates/refineui-tokens/contracts).

The JSON files are canonical and hand-editable. This script:

  contracts.py bootstrap        create contracts that do not exist yet (from
                                the table below), never touching existing ones
  contracts.py refresh          re-derive the auto fields of every contract:
                                `tokens` (scanned from the Rust source), the
                                density `sizes` numbers (from density.json) and
                                each state's implemented/planned status
  contracts.py index            rewrite contracts/index.rs (embedded list)

Run `cargo test -p refineui-tokens` / `cargo test -p refineui` afterwards:
the validator checks schema shape, coverage of every public component and
that embedded sizes equal the Rust density metrics.
"""
import json, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CDIR = os.path.join(ROOT, 'crates/refineui-tokens/contracts')
COMP = os.path.join(CDIR, 'components')
SRC = os.path.join(ROOT, 'crates/refineui/src')
STATES = ['default', 'hovered', 'pressed', 'focused', 'disabled', 'loading', 'selected', 'dragging']

# Archetype → applicable states with the visual contract.
ARCH = {
    'action': {
        'hovered': 'hover fill (`*Hover` alias) over duration-fast',
        'pressed': 'active fill (`*Active` alias)',
        'focused': '2px focusRing (borderFocus) on :focus-visible only',
        'disabled': 'foregroundDisabled / backgroundSurfaceDisabled, no pointer',
        'loading': 'spinner replaces the leading icon; not operable',
        'selected': 'toggled / pressed-in fill when used as a toggle',
    },
    'toggle': {
        'hovered': 'hover border / fill', 'pressed': 'active fill',
        'focused': 'focusRing on :focus-visible', 'disabled': 'disabled aliases',
        'selected': 'checked / on: backgroundBrand + foregroundOnBrand',
    },
    'field': {
        'hovered': 'borderHover', 'pressed': None,
        'focused': 'borderFocus + focusRing; caret visible', 'disabled': 'backgroundSurfaceDisabled, foregroundDisabled',
        'loading': 'trailing spinner (async validation / search)', 'selected': 'text selection uses the selection highlight',
    },
    'select': {
        'hovered': 'borderHover', 'pressed': 'opens the list', 'focused': 'focusRing',
        'disabled': 'disabled aliases', 'loading': 'trailing spinner while options load',
        'selected': 'chosen option shows a check; trigger shows its label',
    },
    'collection': {
        'hovered': 'row plate backgroundPrimaryHover (pointer)', 'pressed': 'row plate backgroundPrimaryActive',
        'focused': 'keyboard cursor row plate + focusRing on the container', 'disabled': 'row foregroundDisabled, skipped by arrows',
        'loading': 'skeleton rows / trailing spinner', 'selected': 'selected plate (backgroundPrimaryHover / surfaceSelected) + medium weight',
        'dragging': 'row lifts (dragPreview layer); drop indicator between rows',
    },
    'overlay': {
        'focused': 'first focusable child focused on open; focus restored on close',
    },
    'container': {
        'hovered': 'trigger hover fill', 'pressed': 'trigger active fill', 'focused': 'focusRing on the trigger',
        'disabled': 'trigger disabled aliases', 'selected': 'expanded / open',
    },
    'slider': {
        'hovered': 'thumb hover', 'pressed': 'thumb grabbed', 'focused': 'focusRing on the thumb',
        'disabled': 'disabled aliases', 'dragging': 'thumb follows the pointer',
    },
    'splitter': {
        'hovered': 'handle line borderStrong; resize cursor', 'pressed': 'handle line borderFocus',
        'focused': 'focusRing on the handle', 'disabled': 'fixed size, no cursor', 'dragging': 'panels resize live',
    },
    'group': {
        'focused': 'roving focus among the child buttons; each child follows the Button contract',
        'disabled': 'disables every child',
    },
    'display': {},
    'drag': {
        'hovered': 'dots foregroundTertiary → foregroundSecondary; grab cursor',
        'pressed': 'grabbing cursor',
        'dragging': 'DragPreview chip follows the pointer; DropIndicator marks the insertion point',
    },
}
KEYS = {
    'action': [('enter', 'activate'), ('space', 'activate')],
    'toggle': [('space', 'toggle')],
    'radio': [('up', 'select previous'), ('down', 'select next'), ('left', 'select previous'), ('right', 'select next')],
    'field': [('secondary-a', 'select all'), ('secondary-c', 'copy'), ('secondary-x', 'cut'), ('secondary-v', 'paste'),
              ('secondary-z', 'undo'), ('shift-left', 'extend selection'), ('home', 'line start'), ('end', 'line end')],
    'select': [('enter', 'open / choose'), ('space', 'open'), ('down', 'open / next option'), ('up', 'previous option'),
               ('escape', 'close'), ('home', 'first option'), ('end', 'last option')],
    'collection': [('down', 'next item'), ('up', 'previous item'), ('home', 'first item'), ('end', 'last item'),
                   ('enter', 'activate item')],
    'menu': [('down', 'next item'), ('up', 'previous item'), ('right', 'open submenu'), ('left', 'close submenu'),
             ('enter', 'activate item'), ('escape', 'close')],
    'tree': [('down', 'next row'), ('up', 'previous row'), ('right', 'expand / first child'), ('left', 'collapse / parent'),
             ('enter', 'activate row'), ('shift-down', 'extend selection'), ('secondary-a', 'select all')],
    'tabs': [('left', 'previous tab'), ('right', 'next tab'), ('home', 'first tab'), ('end', 'last tab')],
    'overlay': [('escape', 'dismiss')],
    'dialog': [('escape', 'dismiss'), ('tab', 'next focusable (trapped)'), ('shift-tab', 'previous focusable (trapped)')],
    'slider': [('right', 'increase step'), ('left', 'decrease step'), ('up', 'increase step'), ('down', 'decrease step'),
               ('pageup', 'increase 10 steps'), ('pagedown', 'decrease 10 steps'), ('home', 'minimum'), ('end', 'maximum')],
    'spin': [('up', 'increment'), ('down', 'decrement'), ('pageup', 'increment ×10'), ('pagedown', 'decrement ×10')],
    'container': [('enter', 'toggle'), ('space', 'toggle')],
    'command': [('down', 'next result'), ('up', 'previous result'), ('enter', 'run result'), ('escape', 'close')],
    'table': [('down', 'next row'), ('up', 'previous row'), ('shift-down', 'extend selection'), ('secondary-a', 'select all'),
              ('enter', 'open row')],
    'splitter': [('left', 'shrink leading panel'), ('right', 'grow leading panel'), ('home', 'collapse'), ('end', 'expand')],
    'otp': [('left', 'previous slot'), ('right', 'next slot'), ('backspace', 'clear and move back'), ('secondary-v', 'paste code')],
    'calendar': [('left', 'previous day'), ('right', 'next day'), ('up', 'previous week'), ('down', 'next week'),
                 ('pageup', 'previous month'), ('pagedown', 'next month'), ('enter', 'choose day')],
    'carousel': [('left', 'previous slide'), ('right', 'next slide')],
    'reorder': [('down', 'next item'), ('up', 'previous item'), ('enter', 'activate item'),
                ('alt-up', 'move item up'), ('alt-down', 'move item down')],
    'menubar': [('f10', 'focus the menu bar'), ('alt', 'focus the menu bar'), ('left', 'previous menu'), ('right', 'next menu'),
                ('down', 'open menu'), ('enter', 'open menu'), ('escape', 'close / leave')],
    'shell': [('f6', 'next region'), ('shift-f6', 'previous region'), ('f10', 'focus the menu bar'),
              ('secondary-shift-p', 'command palette')],
    'selection': [('secondary-a', 'select all'), ('escape', 'clear selection'), ('right', 'next item'), ('left', 'previous item'),
                  ('shift-right', 'extend selection'), ('shift-left', 'extend selection')],
    'none': [],
}
TAB = {'action': 'single', 'toggle': 'single', 'field': 'single', 'select': 'single', 'collection': 'roving',
       'overlay': 'container', 'group': 'roving', 'container': 'single', 'slider': 'single', 'splitter': 'single', 'display': 'none', 'drag': 'none'}
# Desktop convention (Phase 2): controls keep the arrow; only links (web
# content) use the hand. See foundation::CursorRole.
CURSOR = {'action': 'default', 'toggle': 'default', 'field': 'text', 'select': 'default', 'collection': 'default',
          'overlay': 'default', 'group': 'default', 'container': 'default', 'slider': 'default', 'splitter': 'resize-col',
          'display': 'default', 'drag': 'grab'}
HAND = {'link', 'footer', 'navigation-menu', 'carousel'}
# Per-contract tweaks applied on bootstrap.
OVERRIDES = {
    'window-controls': {'focus': {'tab_stop': 'none', 'ring': 'none', 'restores_focus': False},
                        'drop_states': ['focused', 'loading', 'selected']},
    'application-shell': {'focus': {'tab_stop': 'roving', 'ring': 'inset', 'restores_focus': False}},
    'titlebar': {'pointer': {'cursor': 'default', 'hover_only': False}},
}

# id, name, rust types, source files, archetype, keys, role, layer, status, phase, feature, desktop target, metric
C = [
 ('accordion','Accordion',['Accordion'],['accordion'],'container','container','region','components','exists',4,None,None,'content'),
 ('collapsible','Collapsible',['Collapsible'],['accordion','disclosure'],'container','container','group','components','exists',4,None,None,'content'),
 ('alert','Alert',['Alert'],['alert'],'display','none','alert','components','exists',4,None,None,'content'),
 ('avatar','Avatar',['Avatar','Avatars'],['avatar'],'display','none','img','components','exists',4,None,None,'fixed'),
 ('badge','Badge',['Badge'],['badge'],'display','none','status','components','exists',4,None,None,'fixed'),
 ('breadcrumb','Breadcrumb',['Breadcrumb'],['breadcrumb'],'collection','tabs','navigation','components','adapt',4,None,'Path bar',None),
 ('bubble','Bubble',['Bubble','BubbleGroup'],['bubble'],'display','none','article','components','exists',4,None,None,'content'),
 ('button','Button',['Button'],['button'],'action','action','button','components','exists',4,None,None,'button'),
 ('button-group','Button Group',['ButtonGroup'],['button_group'],'group','tabs','group','components','exists',4,None,None,'button'),
 ('calendar','Calendar',['Calendar'],['calendar'],'collection','calendar','grid','components','exists',4,None,None,'fixed'),
 ('card','Card',['Card'],['card'],'action','action','group','components','exists',4,None,None,'content'),
 ('carousel','Carousel',['Carousel'],['carousel'],'collection','carousel','region','components','optional',4,'web-extras',None,'content'),
 ('chart','Chart',['Chart'],['chart'],'display','none','img','components','exists',4,None,None,'content'),
 ('checkbox','Checkbox',['Checkbox'],['checkbox'],'toggle','toggle','checkbox','components','exists',4,None,None,'fixed'),
 ('tag','Tag',['Chip'],['chip'],'action','action','button','components','exists',4,None,None,'chip'),
 ('code-block','Code Block',['CodeBlock'],['code_block'],'display','none','code','components','exists',4,None,'Editor (read-only)','content'),
 ('command','Command',['Command'],['command'],'collection','command','combobox','interaction','adapt',2,None,'Command Palette','row'),
 ('context-menu','Context Menu',['ContextMenu'],['context_menu','menu'],'collection','menu','menu','components','exists',4,None,None,'row'),
 ('date-input','Date Input',['DateInput'],['date_input'],'field','calendar','combobox','components','exists',4,None,None,'field'),
 ('dialog','Dialog',['Dialog'],['dialog'],'overlay','dialog','dialog','components','adapt',4,None,'Dialog / Sheet','content'),
 ('drawer','Drawer',['Drawer'],['dialog'],'overlay','dialog','dialog','components','adapt',4,None,'Sheet / Inspector overlay','content'),
 ('divider','Divider',['Divider'],['divider'],'display','none','separator','components','exists',4,None,None,'fixed'),
 ('field','Field',['Field'],['field'],'display','none','group','components','exists',4,None,None,'content'),
 ('footer','Footer',['Footer'],['footer'],'collection','collection','contentinfo','components','optional',4,'web-extras',None,'content'),
 ('input','Input',['TextInput'],['input'],'field','field','textbox','components','exists',4,None,None,'field'),
 ('textarea','Textarea',['TextInput'],['input'],'field','field','textbox','components','exists',4,None,None,'field'),
 ('search-field','Search Field',['TextInput'],['input'],'field','field','searchbox','components','exists',4,None,None,'field'),
 ('input-otp','Input OTP',['InputOtp'],['input_otp'],'field','otp','group','components','exists',4,None,None,'field'),
 ('kbd','Kbd',['Kbd'],['kbd'],'display','none','none','interaction','adapt',2,None,'Shortcut Hint','fixed'),
 ('link','Link',['Link'],['link'],'action','action','link','components','optional',4,'web-extras',None,'content'),
 ('marker','Marker',['Marker'],['marker'],'display','none','mark','components','exists',4,None,None,'content'),
 ('menu','Menu',['Menu'],['menu'],'collection','menu','menu','components','exists',4,None,None,'row'),
 ('dropdown','Dropdown',['DropdownMenu'],['menu'],'collection','menu','menu','components','exists',4,None,None,'row'),
 ('message','Message',['Message'],['message'],'display','none','article','components','exists',4,None,None,'content'),
 ('navigation-menu','Navigation Menu',['NavigationMenu'],['navigation_menu'],'collection','menu','navigation','components','optional',4,'web-extras',None,'content'),
 ('pagination','Pagination',['Pagination'],['pagination'],'collection','tabs','navigation','components','exists',4,None,None,'button'),
 ('popover','Popover',['Popover'],['popover'],'overlay','overlay','dialog','components','exists',4,None,None,'content'),
 ('progress','Progress',['Progress'],['progress'],'display','none','progressbar','components','exists',4,None,None,'fixed'),
 ('progress-stepper','Progress Stepper',['ProgressStepper'],['progress_stepper'],'display','none','list','components','exists',4,None,None,'content'),
 ('radio','Radio',['Radio','RadioGroup'],['radio'],'toggle','radio','radiogroup','components','exists',4,None,None,'fixed'),
 ('resizable','Resizable',['ResizablePanelGroup'],['resizable'],'splitter','splitter','separator','composition','adapt',3,None,'Split View / Resizable Handle','desktop'),
 ('result','Result',['ResultView'],['result'],'display','none','status','components','adapt',4,None,'Empty State / Error State','content'),
 ('scroll-area','Scroll Area',['ScrollArea','ScrollEdgeItem'],['scroll_area'],'display','none','scrollbar','components','exists',4,None,None,'content'),
 ('segmented-control','Segmented Control',['SegmentedControl'],['segmented_control'],'toggle','tabs','radiogroup','components','exists',4,None,None,'button'),
 ('select','Select',['Select'],['select'],'select','select','combobox','components','exists',4,None,None,'select'),
 ('sidebar','Sidebar',['Sidebar','SidebarPeek'],['sidebar'],'collection','collection','navigation','composition','adapt',3,None,'Panel (navigation)','desktop'),
 ('skeleton','Skeleton',['Skeleton'],['skeleton'],'display','none','none','components','exists',4,None,None,'content'),
 ('slider','Slider',['Slider'],['slider'],'slider','slider','slider','components','exists',4,None,None,'fixed'),
 ('spin-button','Spin Button',['SpinButton'],['spin_button'],'field','spin','spinbutton','components','exists',4,None,None,'field'),
 ('spinner','Spinner',['Spinner'],['spinner'],'display','none','progressbar','components','exists',4,None,None,'fixed'),
 ('stepper','Stepper',['Stepper'],['stepper'],'collection','tabs','list','components','exists',4,None,None,'content'),
 ('switch','Switch',['Switch'],['switch'],'toggle','toggle','switch','components','exists',4,None,None,'fixed'),
 ('table','Table',['Table'],['table'],'collection','table','grid','components','adapt',4,None,'Table (virtualized, multi-select)','row'),
 ('tabs','Tabs',['Tabs'],['tabs'],'collection','tabs','tablist','components','exists',4,None,'Tab Bar (documents)','button'),
 ('text','Text',['Text'],['text'],'display','none','none','foundation','exists',4,None,None,'content'),
 ('label','Label',['Label'],['text'],'display','none','label','components','exists',4,None,None,'content'),
 ('time-input','Time Input',['TimeInput'],['time_input'],'field','spin','combobox','components','exists',4,None,None,'field'),
 ('toast','Toast',['Toast','Toaster'],['toast'],'overlay','overlay','status','components','adapt',4,None,'Notification Center','content'),
 ('toggle-button','Toggle Button',['ToggleButton'],['toggle'],'action','action','button','components','exists',4,None,None,'button'),
 ('toolbar','Toolbar',['Toolbar'],['toolbar'],'group','tabs','toolbar','application','adapt',2,None,'Toolbar (window)','desktop'),
 ('tooltip','Tooltip',['Tooltip'],['tooltip'],'display','none','tooltip','components','exists',4,None,None,'content'),
 ('tree','Tree',['Tree'],['tree'],'collection','tree','tree','components','adapt',4,None,'Tree (virtualized, multi-select, DnD)','row'),
 ('icon','Icon',['Icon'],['../icon'],'display','none','img','foundation','exists',2,None,None,'fixed'),
 # Phase 2 — interaction and shell chrome.
 ('activity-bar','Activity Bar',['ActivityBar','ActivityDrag'],['../shell/activity_bar'],'collection','reorder','navigation','application','new',2,None,'Activity Bar / Navigation Rail','desktop'),
 ('titlebar','Titlebar',['Titlebar','WindowFrame'],['../shell/titlebar'],'display','none','banner','application','new',2,None,'Titlebar','desktop'),
 ('window-controls','Window Controls',['WindowControls'],['../shell/titlebar'],'action','none','group','platform','new',2,None,'Window Controls','fixed'),
 ('menu-bar','Menu Bar',['MenuBar'],['../shell/menu_bar'],'collection','menubar','menubar','application','new',2,None,'Menu Bar','desktop'),
 ('status-bar','Status Bar',['StatusBar'],['../shell/status_bar'],'group','action','status','application','new',2,None,'Status Bar','desktop'),
 ('application-shell','Application Shell',['ApplicationShell'],['../shell/app_shell'],'group','shell','application','application','new',2,None,'Application Shell','desktop'),
 ('shortcut-hint','Shortcut Hint',['ShortcutHint'],['shortcut_hint'],'display','none','none','interaction','new',2,None,'Shortcut Hint','fixed'),
 ('drag-handle','Drag Handle',['DragHandle'],['drag'],'drag','reorder','none','interaction','new',2,None,'Drag Handle','fixed'),
 ('drag-preview','Drag Preview',['DragPreview'],['drag'],'display','none','none','interaction','new',2,None,'Drag Preview','content'),
 ('drop-zone','Drop Zone',['DropIndicator','DropZone'],['drag'],'display','none','none','interaction','new',2,None,'Drop Zone','content'),
 ('selection-box','Selection Box',['SelectionArea'],['selection_box'],'collection','selection','listbox','interaction','new',2,None,'Selection Box','content'),
 ('command-palette','Command Palette',['CommandPalette'],['command_palette','command'],'overlay','command','dialog','application','new',2,None,'Command Palette','content'),
]

def camel(s):
    p = s.lower().split('_'); return p[0] + ''.join(x.capitalize() for x in p[1:])
def pascal(s):
    return ''.join(x.capitalize() for x in s.lower().split('_'))

def scan_tokens(files):
    src = ''
    for f in files:
        p = os.path.join(SRC, 'components', f + '.rs') if not f.startswith('../') else os.path.join(SRC, f[3:] + '.rs')
        if os.path.exists(p): src += open(p).read()
    def found(pat, fmt):
        return sorted({fmt(m) for m in re.findall(pat, src)})
    t = {
        'color': found(r'\bc\.((?:background|foreground|border|surface)_[a-z_]+)', camel),
        'radius': found(r'radius::([A-Z_]+)', lambda m: 'rounded' + pascal(m)),
        'spacing': found(r'spacing::([A-Z_]+)', lambda m: 'size' + pascal(m)),
        'typography': found(r'role::([A-Z_]+)', camel),
        'motion': found(r'Motion::([A-Z_]+)', lambda m: 'motion.' + camel(m)),
        'elevation': found(r'Elevation::([A-Z_]+)', lambda m: 'elevation.' + camel(m)),
        'stroke': found(r'stroke::([A-Z_]+)', lambda m: 'strokeWidth' + pascal(m)),
    }
    if 'focus_ring' in src: t['color'] = sorted(set(t['color']) | {'borderFocus'})
    return {k: v for k, v in t.items() if v}, src

DETECT = {
    'hovered': r'hovered|track_interaction|\.hover\(',
    'pressed': r'pressed|\.active\(',
    'focused': r'focus_ring|focus_visible|track_focus',
    'disabled': r'disabled',
    'loading': r'loading',
    'selected': r'fn selected|fn checked|selected:|checked:|fn value|fn open\b|on: bool|pressed: bool',
    'dragging': r'on_drag|dragging',
}

def states_for(arch, src, old=None):
    out = {'default': {'status': 'implemented', 'visual': 'resting tokens'}}
    for s in STATES[1:]:
        vis = ARCH[arch].get(s, None) if s in ARCH[arch] else None
        if vis is None and not (s in ARCH[arch]):
            out[s] = None; continue
        if vis is None: out[s] = None; continue
        status = 'implemented' if re.search(DETECT[s], src) else 'planned'
        o = (old or {}).get(s) or {}
        out[s] = {'status': status, 'visual': o.get('visual', vis)}
    return out

def sizes_for(metric):
    d = json.load(open(os.path.join(CDIR, 'density.json')))
    if metric in ('button', 'field', 'select', 'chip'):
        return {'metric': metric, 'sizes': ['sm', 'md', 'lg'], **{k: d[k][metric] for k in ('compact', 'comfortable', 'spacious')}}
    if metric == 'row':
        return {'metric': 'row', **{k: {'row_height': d[k]['desktop']['row_height']} for k in ('compact', 'comfortable', 'spacious')}}
    if metric == 'desktop':
        return {'metric': 'desktop', **{k: d[k]['desktop'] for k in ('compact', 'comfortable', 'spacious')}}
    return {'metric': metric or 'content'}

def build(row):
    (cid, name, types, files, arch, keys, role, layer, status, phase, feature, target, metric) = row
    tokens, src = scan_tokens(files)
    tab = TAB[arch]
    if keys == 'dialog': tab = 'trap'
    return {
        '$schema': '../schema.json', 'id': cid, 'name': name, 'layer': layer, 'status': status, 'phase': phase,
        'feature': feature, 'desktop_target': target, 'rust_types': types, 'a11y_role': role,
        'states': states_for(arch, src), 'sizes': sizes_for(metric if metric else 'content'), 'tokens': tokens,
        'keyboard': [{'keys': k, 'action': a} for k, a in KEYS[keys]],
        'focus': {'tab_stop': tab, 'ring': 'none' if tab in ('none', 'container', 'trap') else 'focusRing',
                  'restores_focus': arch == 'overlay'},
        'pointer': {'cursor': 'pointer' if cid in HAND else CURSOR[arch], 'hover_only': False},
        **{k: v for k, v in OVERRIDES.get(cid, {}).items() if k != 'drop_states'},
    }

def _drop(c):
    for st in OVERRIDES.get(c['id'], {}).get('drop_states', []):
        c['states'][st] = None
    return c

def write(c):
    with open(os.path.join(COMP, c['id'] + '.json'), 'w') as f:
        json.dump(c, f, indent=2, ensure_ascii=False); f.write('\n')

def index():
    ids = sorted(f[:-5] for f in os.listdir(COMP) if f.endswith('.json'))
    lines = ['// @generated by scripts/contracts.py index — do not edit.', '/// Every embedded contract, sorted by id.', 'pub const ALL: &[ContractFile] = &[']
    for i in ids:
        lines.append(f'    ContractFile {{\n        id: "{i}",\n        json: include_str!("components/{i}.json"),\n    }},')
    lines.append('];')
    open(os.path.join(CDIR, 'index.rs'), 'w').write('\n'.join(lines) + '\n')

def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else 'refresh'
    os.makedirs(COMP, exist_ok=True)
    rows = {r[0]: r for r in C}
    if cmd == 'bootstrap':
        for r in C:
            if not os.path.exists(os.path.join(COMP, r[0] + '.json')): write(_drop(build(r)))
    elif cmd == 'refresh':
        for f in sorted(os.listdir(COMP)):
            p = os.path.join(COMP, f); c = json.load(open(p)); r = rows.get(c['id'])
            if not r: continue
            fresh = build(r)
            c['tokens'] = fresh['tokens']
            c['sizes'] = sizes_for(c['sizes'].get('metric'))
            c['pointer']['cursor'] = fresh['pointer']['cursor']
            _, src = scan_tokens(r[3])
            for s in STATES[1:]:
                if c['states'].get(s) and fresh['states'].get(s):
                    c['states'][s]['status'] = fresh['states'][s]['status']
            write(c)
    elif cmd != 'index':
        sys.exit(__doc__)
    index()

if __name__ == '__main__':
    main()
