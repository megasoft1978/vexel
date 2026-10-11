#import <Cocoa/Cocoa.h>

#include "platform/platform_text_ui.h"
#include "platform_window_internal.h"
#include "app/viewer_commands.h"

#include <string.h>

#define PLATFORM_RECENT_LIMIT 12U

@interface VexelApplicationDelegate : NSObject <NSApplicationDelegate>
@property(nonatomic, assign) platform_window *owner;
- (void)actionOpenDocument:(id)sender;
- (void)actionNewWindow:(id)sender;
- (void)actionRunCommand:(id)sender;
- (void)actionReload:(id)sender;
- (void)actionToggleCheckerboard:(id)sender;
- (void)actionCycleBackground:(id)sender;
- (void)actionFitWidth:(id)sender;
- (void)actionFitWindow:(id)sender;
- (void)actionActualSize:(id)sender;
- (void)actionTogglePixelGrid:(id)sender;
- (void)actionToggleRulers:(id)sender;
- (void)actionToggleGuides:(id)sender;
- (void)actionToggleGuideLock:(id)sender;
- (void)actionZoomToSelection:(id)sender;
- (void)actionClearGuides:(id)sender;
- (void)actionUIScale:(id)sender;
- (void)actionQuit:(id)sender;
- (void)actionOpenRecent:(id)sender;
- (void)actionClearRecent:(id)sender;
@end

static __strong VexelApplicationDelegate *platform_delegate;
static NSMenu *platform_recent_menu;

static NSMenuItem *platform_menu_item(NSString *title, NSString *key, SEL action,
                                      NSInteger tag)
{
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:action
        keyEquivalent:key];
    item.target = platform_delegate;
    item.tag = tag;
    item.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    return item;
}

static NSString *platform_command_string(const char *text)
{
    if (text == NULL) return @"";
    NSString *value = [NSString stringWithUTF8String:text];
    return value == nil ? @"" : value;
}

static NSMenuItem *platform_command_menu_item(const viewer_command_descriptor *command)
{
    if (command == NULL) return nil;
    NSString *key = platform_command_string(command->menu_key);
    NSMenuItem *item = [[NSMenuItem alloc]
        initWithTitle:platform_command_string(command->title)
        action:@selector(actionRunCommand:) keyEquivalent:key];
    item.target = platform_delegate;
    item.tag = (NSInteger)command->id;
    NSEventModifierFlags flags = 0U;
    if ((command->menu_modifiers & VIEWER_COMMAND_MENU_COMMAND) != 0U)
        flags |= NSEventModifierFlagCommand;
    if ((command->menu_modifiers & VIEWER_COMMAND_MENU_SHIFT) != 0U)
        flags |= NSEventModifierFlagShift;
    if ((command->menu_modifiers & VIEWER_COMMAND_MENU_OPTION) != 0U)
        flags |= NSEventModifierFlagOption;
    item.keyEquivalentModifierMask = flags;
    return item;
}

static void platform_add_command_group(NSMenu *menu, viewer_command_group group)
{
    for (size_t index = 0U; index < viewer_command_count(); ++index) {
        const viewer_command_descriptor *command = viewer_command_at(index);
        if (command == NULL || command->group != group ||
            command->id == VIEWER_COMMAND_QUIT || command->id == VIEWER_COMMAND_SEARCH)
            continue;
        [menu addItem:platform_command_menu_item(command)];
    }
}

static NSString *platform_shortcut_title(const viewer_command_descriptor *command)
{
    if (command == NULL) return @"";
    NSString *title = platform_command_string(command->title);
    if (command->shortcut[0] == '\0') return title;
    return [NSString stringWithFormat:@"%@  %@", title,
        platform_command_string(command->shortcut)];
}

static void platform_add_shortcut_discovery(NSMenu *menu)
{
    static const viewer_command_group groups[] = {
        VIEWER_COMMAND_GROUP_TOOLS, VIEWER_COMMAND_GROUP_SELECTION,
        VIEWER_COMMAND_GROUP_CANVAS, VIEWER_COMMAND_GROUP_DOCUMENT
    };
    for (size_t group_index = 0U;
         group_index < sizeof(groups) / sizeof(groups[0]); ++group_index) {
        NSMenuItem *group_item = [[NSMenuItem alloc]
            initWithTitle:platform_command_string(
                viewer_command_group_name(groups[group_index]))
            action:nil keyEquivalent:@""];
        NSMenu *submenu = [[NSMenu alloc]
            initWithTitle:platform_command_string(
                viewer_command_group_name(groups[group_index]))];
        for (size_t index = 0U; index < viewer_command_count(); ++index) {
            const viewer_command_descriptor *command = viewer_command_at(index);
            if (command == NULL || command->group != groups[group_index]) continue;
            NSMenuItem *shortcut = [[NSMenuItem alloc]
                initWithTitle:platform_shortcut_title(command)
                action:nil keyEquivalent:@""];
            shortcut.enabled = NO;
            [submenu addItem:shortcut];
        }
        group_item.submenu = submenu;
        [menu addItem:group_item];
    }
}

static bool path_is_svg(const char *path, size_t length)
{
    return platform_path_supported_internal(path, length);
}

static void platform_refresh_recent_menu(void)
{
    if (platform_recent_menu == nil) return;
    [platform_recent_menu removeAllItems];
    NSArray *paths = [[NSUserDefaults standardUserDefaults] arrayForKey:@"VexelRecentFiles"];
    NSUInteger added = 0U;
    for (id item in paths) {
        if (![item isKindOfClass:[NSString class]]) continue;
        NSString *path = (NSString *)item;
        const char *bytes = path.fileSystemRepresentation;
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        if (!path_is_svg(bytes, length)) continue;
        NSMenuItem *recent = [[NSMenuItem alloc] initWithTitle:path.lastPathComponent
            action:@selector(actionOpenRecent:) keyEquivalent:@""];
        recent.target = platform_delegate;
        recent.representedObject = path;
        [platform_recent_menu addItem:recent];
        if (++added >= PLATFORM_RECENT_LIMIT) break;
    }
    if (added == 0U) {
        NSMenuItem *empty = [[NSMenuItem alloc] initWithTitle:@"No Recent Files"
            action:nil keyEquivalent:@""];
        empty.enabled = NO;
        [platform_recent_menu addItem:empty];
    }
    [platform_recent_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *clear = [[NSMenuItem alloc] initWithTitle:@"Clear Menu"
        action:@selector(actionClearRecent:) keyEquivalent:@""];
    clear.target = platform_delegate;
    [platform_recent_menu addItem:clear];
}

void platform_macos_menus_init(void)
{
    platform_delegate = [[VexelApplicationDelegate alloc] init];
    [NSApp setDelegate:platform_delegate];
    NSMenu *main = [[NSMenu alloc] initWithTitle:@"Main Menu"];

    NSMenuItem *application = [[NSMenuItem alloc] initWithTitle:@"vexel" action:nil keyEquivalent:@""];
    NSMenu *application_menu = [[NSMenu alloc] initWithTitle:@"vexel"];
    [application_menu addItem:[[NSMenuItem alloc] initWithTitle:@"About vexel"
        action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""]];
    [application_menu addItem:[NSMenuItem separatorItem]];
    [application_menu addItem:platform_command_menu_item(
        viewer_command_find(VIEWER_COMMAND_QUIT))];
    application.submenu = application_menu;
    [main addItem:application];

    NSMenuItem *file = [[NSMenuItem alloc] initWithTitle:@"File" action:nil keyEquivalent:@""];
    NSMenu *file_menu = [[NSMenu alloc] initWithTitle:@"File"];
    for (size_t index = 0U; index < viewer_command_count(); ++index) {
        const viewer_command_descriptor *command = viewer_command_at(index);
        if (command != NULL && command->group == VIEWER_COMMAND_GROUP_DOCUMENT &&
            command->id != VIEWER_COMMAND_QUIT && command->id != VIEWER_COMMAND_SEARCH) {
            [file_menu addItem:platform_command_menu_item(command)];
            if (command->id == VIEWER_COMMAND_OPEN) {
                NSMenuItem *new_window = platform_menu_item(@"New Window", @"n",
                    @selector(actionNewWindow:), PLATFORM_ACTION_NEW_WINDOW);
                new_window.keyEquivalentModifierMask = NSEventModifierFlagCommand |
                    NSEventModifierFlagShift;
                [file_menu addItem:new_window];
            }
        }
    }
    [file_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *recent = [[NSMenuItem alloc] initWithTitle:@"Open Recent"
        action:nil keyEquivalent:@""];
    platform_recent_menu = [[NSMenu alloc] initWithTitle:@"Open Recent"];
    recent.submenu = platform_recent_menu;
    [file_menu addItem:recent];
    [file_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *close = [[NSMenuItem alloc] initWithTitle:@"Close Window"
        action:@selector(performClose:) keyEquivalent:@"w"];
    close.target = nil;
    close.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    [file_menu addItem:close];
    file.submenu = file_menu;
    [main addItem:file];

    NSMenuItem *edit = [[NSMenuItem alloc] initWithTitle:@"Edit" action:nil keyEquivalent:@""];
    NSMenu *edit_menu = [[NSMenu alloc] initWithTitle:@"Edit"];
    [edit_menu addItem:platform_command_menu_item(
        viewer_command_find(VIEWER_COMMAND_SEARCH))];
    [edit_menu addItem:[NSMenuItem separatorItem]];
    platform_add_command_group(edit_menu, VIEWER_COMMAND_GROUP_SELECTION);
    edit.submenu = edit_menu;
    [main addItem:edit];

    NSMenuItem *tools = [[NSMenuItem alloc] initWithTitle:@"Tools" action:nil keyEquivalent:@""];
    NSMenu *tools_menu = [[NSMenu alloc] initWithTitle:@"Tools"];
    platform_add_command_group(tools_menu, VIEWER_COMMAND_GROUP_TOOLS);
    tools.submenu = tools_menu;
    [main addItem:tools];

    NSMenuItem *view = [[NSMenuItem alloc] initWithTitle:@"View" action:nil keyEquivalent:@""];
    NSMenu *view_menu = [[NSMenu alloc] initWithTitle:@"View"];
    platform_add_command_group(view_menu, VIEWER_COMMAND_GROUP_CANVAS);
    NSMenuItem *checker = nil;
    for (NSMenuItem *item in view_menu.itemArray)
        if (item.tag == (NSInteger)VIEWER_COMMAND_TOGGLE_CHECKERBOARD) checker = item;
    if (checker != nil)
    checker.state = NSControlStateValueOn;
    [view_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *rulers = platform_menu_item(@"Rulers", @"",
        @selector(actionToggleRulers:), PLATFORM_ACTION_TOGGLE_RULERS);
    rulers.tag = -(NSInteger)PLATFORM_ACTION_TOGGLE_RULERS;
    rulers.state = NSControlStateValueOn;
    [view_menu addItem:rulers];
    NSMenuItem *guides = platform_menu_item(@"Guides", @"",
        @selector(actionToggleGuides:), PLATFORM_ACTION_TOGGLE_GUIDES);
    guides.tag = -(NSInteger)PLATFORM_ACTION_TOGGLE_GUIDES;
    guides.state = NSControlStateValueOn;
    [view_menu addItem:guides];
    NSMenuItem *guide_lock = platform_menu_item(@"Lock Guides", @"",
        @selector(actionToggleGuideLock:), PLATFORM_ACTION_TOGGLE_GUIDE_LOCK);
    guide_lock.tag = -(NSInteger)PLATFORM_ACTION_TOGGLE_GUIDE_LOCK;
    [view_menu addItem:guide_lock];
    [view_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *zoom_selection = platform_menu_item(@"Zoom to Selection", @"",
        @selector(actionZoomToSelection:), PLATFORM_ACTION_ZOOM_TO_SELECTION);
    zoom_selection.tag = -(NSInteger)PLATFORM_ACTION_ZOOM_TO_SELECTION;
    [view_menu addItem:zoom_selection];
    NSMenuItem *scale = [[NSMenuItem alloc] initWithTitle:@"Interface Size"
        action:nil keyEquivalent:@""];
    NSMenu *scale_menu = [[NSMenu alloc] initWithTitle:@"Interface Size"];
    NSArray<NSString *> *scale_titles = @[@"Standard · 100%", @"Large · 110%",
        @"Larger · 120%"];
    NSInteger current_scale = [NSUserDefaults.standardUserDefaults
        integerForKey:@"VexelUIScalePercent"];
    if (current_scale != 100 && current_scale != 110 && current_scale != 120)
        current_scale = 100;
    const platform_viewer_action scale_actions[] = {PLATFORM_ACTION_UI_SCALE_100,
        PLATFORM_ACTION_UI_SCALE_110, PLATFORM_ACTION_UI_SCALE_120};
    const NSInteger scale_values[] = {100, 110, 120};
    for (NSUInteger index = 0U; index < scale_titles.count; ++index) {
        NSMenuItem *item = platform_menu_item(scale_titles[index], @"",
            @selector(actionUIScale:), (NSInteger)scale_actions[index]);
        item.representedObject = @(scale_values[index]);
        item.state = current_scale == scale_values[index] ?
            NSControlStateValueOn : NSControlStateValueOff;
        [scale_menu addItem:item];
    }
    scale.submenu = scale_menu;
    [view_menu addItem:scale];
    view.submenu = view_menu;
    [main addItem:view];

    NSMenuItem *help = [[NSMenuItem alloc] initWithTitle:@"Help" action:nil keyEquivalent:@""];
    NSMenu *help_menu = [[NSMenu alloc] initWithTitle:@"Help"];
    NSMenuItem *shortcuts = [[NSMenuItem alloc] initWithTitle:@"Keyboard Shortcuts"
        action:nil keyEquivalent:@""];
    NSMenu *shortcuts_menu = [[NSMenu alloc] initWithTitle:@"Keyboard Shortcuts"];
    platform_add_shortcut_discovery(shortcuts_menu);
    shortcuts.submenu = shortcuts_menu;
    [help_menu addItem:shortcuts];
    help.submenu = help_menu;
    [main addItem:help];

    NSMenuItem *window_item = [[NSMenuItem alloc] initWithTitle:@"Window" action:nil keyEquivalent:@""];
    NSMenu *window_menu = [[NSMenu alloc] initWithTitle:@"Window"];
    NSMenuItem *previous_tab = [[NSMenuItem alloc] initWithTitle:@"Show Previous Tab"
        action:@selector(selectPreviousTab:) keyEquivalent:@"["];
    previous_tab.target = nil;
    previous_tab.keyEquivalentModifierMask = NSEventModifierFlagCommand | NSEventModifierFlagShift;
    [window_menu addItem:previous_tab];
    NSMenuItem *next_tab = [[NSMenuItem alloc] initWithTitle:@"Show Next Tab"
        action:@selector(selectNextTab:) keyEquivalent:@"]"];
    next_tab.target = nil;
    next_tab.keyEquivalentModifierMask = NSEventModifierFlagCommand | NSEventModifierFlagShift;
    [window_menu addItem:next_tab];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Move Tab to New Window"
        action:@selector(moveTabToNewWindow:) keyEquivalent:@""]];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Merge All Windows"
        action:@selector(mergeAllWindows:) keyEquivalent:@""]];
    [window_menu addItem:[NSMenuItem separatorItem]];
    NSMenuItem *minimize = [[NSMenuItem alloc] initWithTitle:@"Minimize"
        action:@selector(performMiniaturize:) keyEquivalent:@"m"];
    minimize.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    [window_menu addItem:minimize];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Bring All to Front"
        action:@selector(arrangeInFront:) keyEquivalent:@""]];
    window_item.submenu = window_menu;
    [main addItem:window_item];
    [NSApp setMainMenu:main];
    [NSApp setWindowsMenu:window_menu];
    platform_refresh_recent_menu();
}

void platform_macos_menus_set_owner(platform_window *window)
{
    platform_delegate.owner = window;
}

void platform_macos_menus_clear_owner(platform_window *window)
{
    if (platform_delegate.owner == window) platform_delegate.owner = NULL;
}

bool platform_macos_menus_add_recent(const char *path, size_t length)
{
    if (!path_is_svg(path, length)) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
                                             encoding:NSUTF8StringEncoding];
    if (value == nil) return false;
    value = value.stringByStandardizingPath;
    NSArray *prior = [[NSUserDefaults standardUserDefaults] arrayForKey:@"VexelRecentFiles"];
    NSMutableArray<NSString *> *recent = [NSMutableArray arrayWithObject:value];
    for (id item in prior) {
        if (![item isKindOfClass:[NSString class]] || [(NSString *)item isEqualToString:value]) continue;
        [recent addObject:(NSString *)item];
        if (recent.count >= PLATFORM_RECENT_LIMIT) break;
    }
    [[NSUserDefaults standardUserDefaults] setObject:recent forKey:@"VexelRecentFiles"];
    platform_refresh_recent_menu();
    return true;
}

static size_t copy_utf8(char *destination, size_t capacity, const char *source,
                        size_t source_limit)
{
    if (destination == NULL || capacity == 0U || source == NULL) return 0U;
    size_t amount = strnlen(source, source_limit);
    if (amount >= capacity) amount = capacity - 1U;
    while (amount > 0U && (((const uint8_t *)source)[amount] & 0xc0U) == 0x80U) amount--;
    memcpy(destination, source, amount);
    destination[amount] = '\0';
    return amount;
}

size_t platform_macos_menus_copy_recent(platform_recent_file *out_files, size_t capacity)
{
    if (out_files == NULL || capacity == 0U) return 0U;
    if (capacity > PLATFORM_RECENT_FILES_MAX) capacity = PLATFORM_RECENT_FILES_MAX;
    NSArray *paths = [[NSUserDefaults standardUserDefaults] arrayForKey:@"VexelRecentFiles"];
    size_t added = 0U;
    for (id item in paths) {
        if (![item isKindOfClass:[NSString class]]) continue;
        NSString *path = (NSString *)item;
        const char *path_bytes = path.fileSystemRepresentation;
        size_t path_length = path_bytes == NULL ? 0U : strnlen(path_bytes, PLATFORM_PATH_CAP);
        if (!path_is_svg(path_bytes, path_length) || path_length >= PLATFORM_PATH_CAP) continue;
        platform_recent_file *file = &out_files[added];
        memset(file, 0, sizeof(*file));
        memcpy(file->path, path_bytes, path_length);
        file->path[path_length] = '\0';
        file->path_length = path_length;
        const char *name = path.lastPathComponent.fileSystemRepresentation;
        if (name == NULL) name = "SVG document";
        (void)copy_utf8(file->display_name, sizeof(file->display_name), name,
            PLATFORM_PATH_CAP);
        if (++added >= capacity) break;
    }
    return added;
}

bool platform_macos_menus_set_option_state(platform_window *window,
                                           platform_viewer_action action, bool enabled)
{
    viewer_command_id command = VIEWER_COMMAND_NONE;
    NSInteger custom_tag = 0;
    if (action == PLATFORM_ACTION_TOGGLE_CHECKERBOARD)
        command = VIEWER_COMMAND_TOGGLE_CHECKERBOARD;
    else if (action == PLATFORM_ACTION_TOGGLE_PIXEL_GRID)
        command = VIEWER_COMMAND_TOGGLE_PIXEL_GRID;
    else if (action == PLATFORM_ACTION_TOGGLE_RULERS ||
             action == PLATFORM_ACTION_TOGGLE_GUIDES ||
             action == PLATFORM_ACTION_TOGGLE_GUIDE_LOCK ||
             action == PLATFORM_ACTION_ZOOM_TO_SELECTION)
        custom_tag = -(NSInteger)action;
    if (custom_tag != 0) {
        if (window == NULL || platform_delegate.owner != window) return false;
        NSMenuItem *view = [[NSApp mainMenu] itemWithTitle:@"View"];
        for (NSMenuItem *item in view.submenu.itemArray) {
            if (item.tag == custom_tag) {
                item.state = enabled ? NSControlStateValueOn : NSControlStateValueOff;
                return true;
            }
        }
        return false;
    }
    return platform_macos_menus_set_command_state(window, (uint32_t)command, enabled);
}

bool platform_macos_menus_set_command_state(platform_window *window,
                                           uint32_t command_id, bool enabled)
{
    if (window == NULL || platform_delegate.owner != window || command_id == 0U)
        return false;
    NSMenuItem *view = [[NSApp mainMenu] itemWithTitle:@"View"];
    for (NSMenuItem *item in view.submenu.itemArray) {
        if (item.tag == (NSInteger)command_id) {
            item.state = enabled ? NSControlStateValueOn : NSControlStateValueOff;
            return true;
        }
    }
    return false;
}

bool platform_macos_menus_show_ruler_context_menu(platform_window *window,
    bool horizontal_axis, int32_t logical_x, int32_t logical_y)
{
    if (window == NULL || platform_delegate.owner != window ||
        ![NSThread isMainThread]) return false;
    NSView *view = (__bridge NSView *)platform_window_native_view(window);
    if (view == nil || logical_x < 0 || logical_y < 0) return false;
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"Ruler" ];
    platform_viewer_action action = horizontal_axis ?
        PLATFORM_ACTION_CLEAR_HORIZONTAL_GUIDES : PLATFORM_ACTION_CLEAR_VERTICAL_GUIDES;
    NSString *title = horizontal_axis ? @"Clear Horizontal Guides" :
        @"Clear Vertical Guides";
    NSMenuItem *clear = [[NSMenuItem alloc] initWithTitle:title
        action:@selector(actionClearGuides:) keyEquivalent:@""];
    clear.target = platform_delegate;
    clear.tag = action;
    [menu addItem:clear];
    NSPoint point = NSMakePoint(logical_x, NSHeight(view.bounds) - logical_y);
    return [menu popUpMenuPositioningItem:nil atLocation:point inView:view];
}

@implementation VexelApplicationDelegate
- (void)actionRunCommand:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    NSInteger tag = [(NSMenuItem *)sender tag];
    if (tag > 0 && tag < (NSInteger)VIEWER_COMMAND_COUNT)
        platform_window_dispatch_command(self.owner, (uint32_t)tag);
}
- (void)actionOpenDocument:(id)sender
{
    (void)sender;
    if (self.owner != NULL) (void)platform_window_open_panel(self.owner);
}
- (void)actionNewWindow:(id)sender
{
    (void)sender;
    platform_window_dispatch_action(self.owner, PLATFORM_ACTION_NEW_WINDOW);
}
- (void)actionReload:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_RELOAD); }
- (void)actionToggleCheckerboard:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_CHECKERBOARD); }
- (void)actionCycleBackground:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_CYCLE_BACKGROUND); }
- (void)actionFitWidth:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_FIT_WIDTH); }
- (void)actionFitWindow:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_FIT_WINDOW); }
- (void)actionActualSize:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_ACTUAL_SIZE); }
- (void)actionTogglePixelGrid:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_PIXEL_GRID); }
- (void)actionToggleRulers:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_RULERS); }
- (void)actionToggleGuides:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_GUIDES); }
- (void)actionToggleGuideLock:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_GUIDE_LOCK); }
- (void)actionZoomToSelection:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_ZOOM_TO_SELECTION); }
- (void)actionClearGuides:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    platform_window_dispatch_action(self.owner, (platform_viewer_action)[(NSMenuItem *)sender tag]);
}
- (void)actionUIScale:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    NSMenuItem *selected = (NSMenuItem *)sender;
    NSNumber *value = [selected.representedObject isKindOfClass:[NSNumber class]] ?
        selected.representedObject : nil;
    if (value == nil) return;
    NSInteger percent = value.integerValue;
    if (percent != 100 && percent != 110 && percent != 120) return;
    if (!platform_text_ui_set_ui_scale((uint16_t)percent)) return;
    for (NSMenuItem *item in selected.menu.itemArray)
        item.state = item == selected ? NSControlStateValueOn : NSControlStateValueOff;
    platform_window_dispatch_action(self.owner,
        percent == 110 ? PLATFORM_ACTION_UI_SCALE_110 :
        percent == 120 ? PLATFORM_ACTION_UI_SCALE_120 : PLATFORM_ACTION_UI_SCALE_100);
}
- (void)actionQuit:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_QUIT); }
- (void)actionOpenRecent:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    NSString *path = [(NSMenuItem *)sender representedObject];
    if (path == nil) return;
    const char *bytes = path.fileSystemRepresentation;
    size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
    (void)platform_window_dispatch_file_open(self.owner, bytes, length);
}
- (void)actionClearRecent:(id)sender
{
    (void)sender;
    [[NSUserDefaults standardUserDefaults] removeObjectForKey:@"VexelRecentFiles"];
    platform_refresh_recent_menu();
}
- (void)application:(NSApplication *)application openFiles:(NSArray<NSString *> *)filenames
{
    (void)application;
    BOOL accepted = NO;
    for (NSString *path in filenames) {
        const char *bytes = path.fileSystemRepresentation;
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        accepted |= self.owner == NULL ?
            platform_window_dispatch_file_open(NULL, bytes, length) :
            platform_window_dispatch_new_file(self.owner, bytes, length);
    }
    [NSApp replyToOpenOrPrint:accepted ? NSApplicationDelegateReplySuccess :
                                         NSApplicationDelegateReplyFailure];
}
- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls
{
    (void)application;
    for (NSURL *url in urls) {
        if (!url.isFileURL) continue;
        const char *bytes = url.path.fileSystemRepresentation;
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        if (self.owner == NULL)
            (void)platform_window_dispatch_file_open(NULL, bytes, length);
        else
            (void)platform_window_dispatch_new_file(self.owner, bytes, length);
    }
}
@end
