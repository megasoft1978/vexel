#import <Cocoa/Cocoa.h>

#include "app/viewer_commands.h"
#include "app/platform_command_policy.h"
#include "platform/macos/platform_clipboard.h"
#include "platform/platform_feedback.h"
#include "platform/platform_recents.h"
#include "platform/platform_settings.h"
#include "platform/platform_text_ui.h"
#include "platform_window_internal.h"

#include <string.h>

@interface VexelApplicationDelegate : NSObject <NSApplicationDelegate, NSMenuDelegate>
@property(nonatomic, assign) platform_window *owner;
- (void)dispatchCommand:(id)sender;
- (void)actionClearGuides:(id)sender;
- (void)openDocument:(id)sender;
- (void)openRecent:(id)sender;
- (void)clearRecent:(id)sender;
- (void)showSettings:(id)sender;
- (void)showHelp:(id)sender;
- (void)showFeedback:(id)sender;
- (void)copyDiagnostics:(id)sender;
@end

static __strong VexelApplicationDelegate *platform_delegate;
static NSMenu *platform_recent_menu;
static NSMenuItem *platform_recent_parent_item;
static NSMenuItem *platform_full_screen_item;
static NSMenu *platform_edit_menu;
static platform_command_state platform_command_values;

static NSMenuItem *platform_menu_item(NSString *title, NSString *key, NSEventModifierFlags mask,
                                      SEL action, platform_viewer_action command)
{
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:action keyEquivalent:key];
    item.target = platform_delegate;
    item.tag = (NSInteger)command;
    item.keyEquivalentModifierMask = mask;
    return item;
}

static void platform_add_separator(NSMenu *menu)
{
    [menu addItem:[NSMenuItem separatorItem]];
}

static void platform_add_shortcut_discovery(NSMenu *menu)
{
    for (size_t index = 0U; index < viewer_command_count(); ++index) {
        const viewer_command_descriptor *command = viewer_command_at(index);
        if (command == NULL || command->shortcut[0] == '\0') continue;
        NSString *title = [NSString stringWithUTF8String:command->title];
        NSString *shortcut = [NSString stringWithUTF8String:command->shortcut];
        if (title == nil || shortcut == nil) continue;
        NSMenuItem *item = [[NSMenuItem alloc]
            initWithTitle:[NSString stringWithFormat:@"%@  %@", title, shortcut]
            action:nil keyEquivalent:@""];
        item.enabled = NO;
        [menu addItem:item];
    }
}

static NSMenuItem *platform_menu_heading(NSString *title, NSMenu *parent)
{
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:nil keyEquivalent:@""];
    item.submenu = parent;
    return item;
}

static void platform_append_recent_items(NSMenu *menu)
{
    [menu removeAllItems];
    size_t count = platform_recents_count();
    for (size_t index = 0U; index < count && index < WELCOME_RECENT_LIMIT; ++index) {
        platform_recent_item recent = {0};
        if (!platform_recents_get(index, &recent)) continue;
        NSString *path = [[NSString alloc] initWithBytes:recent.path length:recent.path_length
            encoding:NSUTF8StringEncoding];
        if (path == nil) continue;
        NSString *title = path.lastPathComponent;
        if (!recent.available) title = [title stringByAppendingString:@" — Unavailable"];
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title
            action:@selector(openRecent:) keyEquivalent:@""];
        item.target = platform_delegate;
        item.representedObject = path;
        [menu addItem:item];
    }
    if (count == 0U) {
        NSMenuItem *empty = [[NSMenuItem alloc] initWithTitle:@"No Recent Documents"
            action:nil keyEquivalent:@""];
        empty.enabled = NO;
        [menu addItem:empty];
    }
    platform_add_separator(menu);
    NSMenuItem *clear = [[NSMenuItem alloc] initWithTitle:@"Clear Recent Documents"
        action:@selector(clearRecent:) keyEquivalent:@""];
    clear.target = platform_delegate;
    clear.enabled = count > 0U;
    [menu addItem:clear];
}

static void platform_update_menu(NSMenu *menu, const platform_command_state *state)
{
    for (NSMenuItem *item in menu.itemArray) {
        if (item.submenu != nil) platform_update_menu(item.submenu, state);
        if (item.tag <= PLATFORM_ACTION_NONE || item.tag > PLATFORM_ACTION_REFRESH_PREFERENCES)
            continue;
        platform_viewer_action action = (platform_viewer_action)item.tag;
        item.enabled = platform_command_is_enabled(action, state);
        if (action == PLATFORM_ACTION_TOGGLE_CHECKERBOARD)
            item.state = state->checkerboard_enabled ? NSControlStateValueOn : NSControlStateValueOff;
        else if (action == PLATFORM_ACTION_TOGGLE_PIXEL_GRID)
            item.state = state->pixel_grid_enabled ? NSControlStateValueOn : NSControlStateValueOff;
        else if (action == PLATFORM_ACTION_TOGGLE_STRUCTURE)
            item.state = state->structure_visible ? NSControlStateValueOn : NSControlStateValueOff;
        else if (action == PLATFORM_ACTION_TOGGLE_INSPECTOR)
            item.state = state->inspector_visible ? NSControlStateValueOn : NSControlStateValueOff;
    }
}

void platform_macos_menus_init(void)
{
    if (platform_delegate != nil) return;
    platform_recents_init();
    platform_delegate = [[VexelApplicationDelegate alloc] init];
    [NSApp setDelegate:platform_delegate];
    NSMenu *main = [[NSMenu alloc] initWithTitle:@"Main Menu"];
    NSString *app_name = NSBundle.mainBundle.infoDictionary[@"CFBundleDisplayName"];
    if (app_name.length == 0U) app_name = @"Vexel";

    NSMenu *application_menu = [[NSMenu alloc] initWithTitle:app_name];
    [application_menu addItem:[[NSMenuItem alloc] initWithTitle:[NSString stringWithFormat:@"About %@", app_name]
        action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""]];
    [application_menu addItem:platform_menu_item(@"Settings…", @",", NSEventModifierFlagCommand,
        @selector(showSettings:), PLATFORM_ACTION_NONE)];
    platform_add_separator(application_menu);
    NSMenu *services = [[NSMenu alloc] initWithTitle:@"Services"];
    NSMenuItem *services_item = platform_menu_heading(@"Services", services);
    [application_menu addItem:services_item];
    [NSApp setServicesMenu:services];
    platform_add_separator(application_menu);
    NSMenuItem *hide = [[NSMenuItem alloc] initWithTitle:@"Hide Vexel"
        action:@selector(hide:) keyEquivalent:@"h"];
    hide.target = NSApp;
    hide.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    [application_menu addItem:hide];
    NSMenuItem *hide_others = [[NSMenuItem alloc] initWithTitle:@"Hide Others"
        action:@selector(hideOtherApplications:) keyEquivalent:@"h"];
    hide_others.target = NSApp;
    hide_others.keyEquivalentModifierMask = NSEventModifierFlagCommand | NSEventModifierFlagOption;
    [application_menu addItem:hide_others];
    NSMenuItem *show_all = [[NSMenuItem alloc] initWithTitle:@"Show All"
        action:@selector(unhideAllApplications:) keyEquivalent:@""];
    show_all.target = NSApp;
    [application_menu addItem:show_all];
    platform_add_separator(application_menu);
    [application_menu addItem:platform_menu_item([NSString stringWithFormat:@"Quit %@", app_name],
        @"q", NSEventModifierFlagCommand, @selector(dispatchCommand:), PLATFORM_ACTION_QUIT)];
    [main addItem:platform_menu_heading(app_name, application_menu)];

    NSMenu *file_menu = [[NSMenu alloc] initWithTitle:@"File"];
    [file_menu addItem:platform_menu_item(@"New", @"n", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_NEW_DOCUMENT)];
    [file_menu addItem:platform_menu_item(@"Open…", @"o", NSEventModifierFlagCommand,
        @selector(openDocument:), PLATFORM_ACTION_NONE)];
    [file_menu addItem:platform_menu_item(@"New Window", @"n",
        NSEventModifierFlagCommand | NSEventModifierFlagShift,
        @selector(dispatchCommand:), PLATFORM_ACTION_NEW_WINDOW)];
    platform_add_separator(file_menu);
    platform_recent_menu = [[NSMenu alloc] initWithTitle:@"Open Recent"];
    platform_recent_parent_item = platform_menu_heading(@"Open Recent", platform_recent_menu);
    [file_menu addItem:platform_recent_parent_item];
    platform_add_separator(file_menu);
    [file_menu addItem:platform_menu_item(@"Close", @"w", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_CLOSE_DOCUMENT)];
    [file_menu addItem:platform_menu_item(@"Save", @"s", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_SAVE)];
    [file_menu addItem:platform_menu_item(@"Save As…", @"s",
        NSEventModifierFlagCommand | NSEventModifierFlagShift,
        @selector(dispatchCommand:), PLATFORM_ACTION_SAVE_AS)];
    [file_menu addItem:platform_menu_item(@"Reload", @"r", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_RELOAD)];
    [file_menu addItem:platform_menu_item(@"Export PNG…", @"e",
        NSEventModifierFlagCommand | NSEventModifierFlagShift,
        @selector(dispatchCommand:), PLATFORM_ACTION_EXPORT_PNG)];
    [main addItem:platform_menu_heading(@"File", file_menu)];

    NSMenu *edit_menu = [[NSMenu alloc] initWithTitle:@"Edit"];
    platform_edit_menu = edit_menu;
    edit_menu.delegate = platform_delegate;
    [edit_menu addItem:platform_menu_item(@"Undo", @"z", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_UNDO)];
    [edit_menu addItem:platform_menu_item(@"Redo", @"z",
        NSEventModifierFlagCommand | NSEventModifierFlagShift,
        @selector(dispatchCommand:), PLATFORM_ACTION_REDO)];
    platform_add_separator(edit_menu);
    [edit_menu addItem:platform_menu_item(@"Cut", @"x", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_CUT)];
    [edit_menu addItem:platform_menu_item(@"Copy", @"c", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_COPY)];
    [edit_menu addItem:platform_menu_item(@"Paste", @"v", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_PASTE)];
    NSMenuItem *delete_item = platform_menu_item(@"Delete", @"\x7f", 0,
        @selector(dispatchCommand:), PLATFORM_ACTION_DELETE);
    [edit_menu addItem:delete_item];
    platform_add_separator(edit_menu);
    [edit_menu addItem:platform_menu_item(@"Select All", @"a", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_SELECT_ALL)];
    [main addItem:platform_menu_heading(@"Edit", edit_menu)];

    NSMenu *view_menu = [[NSMenu alloc] initWithTitle:@"View"];
    [view_menu addItem:platform_menu_item(@"Structure", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_STRUCTURE)];
    [view_menu addItem:platform_menu_item(@"Inspector", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_INSPECTOR)];
    platform_add_separator(view_menu);
    [view_menu addItem:platform_menu_item(@"Checkerboard", @"t", NSEventModifierFlagCommand,
        @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_CHECKERBOARD)];
    [view_menu addItem:platform_menu_item(@"Cycle Background", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_CYCLE_BACKGROUND)];
    [view_menu addItem:platform_menu_item(@"Pixel Grid", @"g",
        0,
        @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_PIXEL_GRID)];
    platform_add_separator(view_menu);
    [view_menu addItem:platform_menu_item(@"Rulers", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_RULERS)];
    [view_menu addItem:platform_menu_item(@"Guides", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_GUIDES)];
    [view_menu addItem:platform_menu_item(@"Lock Guides", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_TOGGLE_GUIDE_LOCK)];
    [view_menu addItem:platform_menu_item(@"Zoom to Selection", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_ZOOM_TO_SELECTION)];
    NSMenu *scale_menu = [[NSMenu alloc] initWithTitle:@"Interface Size"];
    const NSInteger scale_values[] = {100, 110, 120};
    const platform_viewer_action scale_actions[] = {
        PLATFORM_ACTION_UI_SCALE_100, PLATFORM_ACTION_UI_SCALE_110,
        PLATFORM_ACTION_UI_SCALE_120
    };
    const char *scale_titles[] = {"Standard · 100%", "Large · 110%", "Larger · 120%"};
    platform_text_ui_preferences text_preferences = {0};
    (void)platform_text_ui_get_extended_preferences(&text_preferences);
    for (size_t index = 0U; index < 3U; ++index) {
        NSString *title = [NSString stringWithUTF8String:scale_titles[index]];
        NSMenuItem *item = platform_menu_item(title, @"", 0,
            @selector(dispatchCommand:), scale_actions[index]);
        item.state = text_preferences.ui_scale_percent == (uint16_t)scale_values[index] ?
            NSControlStateValueOn : NSControlStateValueOff;
        [scale_menu addItem:item];
    }
    [view_menu addItem:platform_menu_heading(@"Interface Size", scale_menu)];
    platform_add_separator(view_menu);
    [view_menu addItem:platform_menu_item(@"Fit Width", @"",
        0, @selector(dispatchCommand:), PLATFORM_ACTION_FIT_WIDTH)];
    [view_menu addItem:platform_menu_item(@"Fit in Window", @"0",
        NSEventModifierFlagCommand, @selector(dispatchCommand:), PLATFORM_ACTION_FIT_WINDOW)];
    [view_menu addItem:platform_menu_item(@"Actual Size", @"1",
        NSEventModifierFlagCommand, @selector(dispatchCommand:), PLATFORM_ACTION_ACTUAL_SIZE)];
    [view_menu addItem:platform_menu_item(@"Zoom In", @"+",
        NSEventModifierFlagCommand, @selector(dispatchCommand:), PLATFORM_ACTION_ZOOM_IN)];
    [view_menu addItem:platform_menu_item(@"Zoom Out", @"-",
        NSEventModifierFlagCommand, @selector(dispatchCommand:), PLATFORM_ACTION_ZOOM_OUT)];
    platform_add_separator(view_menu);
    platform_full_screen_item = [[NSMenuItem alloc] initWithTitle:@"Enter Full Screen"
        action:@selector(toggleFullScreen:) keyEquivalent:@"f"];
    platform_full_screen_item.target = nil;
    platform_full_screen_item.keyEquivalentModifierMask =
        NSEventModifierFlagControl | NSEventModifierFlagCommand;
    [view_menu addItem:platform_full_screen_item];
    [main addItem:platform_menu_heading(@"View", view_menu)];

    NSMenu *window_menu = [[NSMenu alloc] initWithTitle:@"Window"];
    NSMenuItem *previous_tab = [[NSMenuItem alloc] initWithTitle:@"Show Previous Tab"
        action:@selector(selectPreviousTab:) keyEquivalent:@"["];
    previous_tab.target = nil;
    previous_tab.keyEquivalentModifierMask = NSEventModifierFlagCommand |
        NSEventModifierFlagControl;
    [window_menu addItem:previous_tab];
    NSMenuItem *next_tab = [[NSMenuItem alloc] initWithTitle:@"Show Next Tab"
        action:@selector(selectNextTab:) keyEquivalent:@"]"];
    next_tab.target = nil;
    next_tab.keyEquivalentModifierMask = NSEventModifierFlagCommand |
        NSEventModifierFlagControl;
    [window_menu addItem:next_tab];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Move Tab to New Window"
        action:@selector(moveTabToNewWindow:) keyEquivalent:@""]];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Merge All Windows"
        action:@selector(mergeAllWindows:) keyEquivalent:@""]];
    platform_add_separator(window_menu);
    NSMenuItem *minimize = [[NSMenuItem alloc] initWithTitle:@"Minimize"
        action:@selector(performMiniaturize:) keyEquivalent:@"m"];
    minimize.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    [window_menu addItem:minimize];
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Zoom"
        action:@selector(performZoom:) keyEquivalent:@""]];
    platform_add_separator(window_menu);
    [window_menu addItem:[[NSMenuItem alloc] initWithTitle:@"Bring All to Front"
        action:@selector(arrangeInFront:) keyEquivalent:@""]];
    [main addItem:platform_menu_heading(@"Window", window_menu)];
    [NSApp setWindowsMenu:window_menu];

    NSMenu *help_menu = [[NSMenu alloc] initWithTitle:@"Help"];
    [help_menu addItem:platform_menu_item(@"Vexel Help", @"?",
        NSEventModifierFlagCommand | NSEventModifierFlagShift,
        @selector(showHelp:), PLATFORM_ACTION_NONE)];
    NSMenu *shortcuts_menu = [[NSMenu alloc] initWithTitle:@"Keyboard Shortcuts"];
    platform_add_shortcut_discovery(shortcuts_menu);
    [help_menu addItem:platform_menu_heading(@"Keyboard Shortcuts", shortcuts_menu)];
    platform_add_separator(help_menu);
    [help_menu addItem:platform_menu_item(@"Send Feedback…", @"",
        0, @selector(showFeedback:), PLATFORM_ACTION_NONE)];
    platform_add_separator(help_menu);
    [help_menu addItem:platform_menu_item(@"Copy Diagnostics", @"",
        0, @selector(copyDiagnostics:), PLATFORM_ACTION_NONE)];
    [main addItem:platform_menu_heading(@"Help", help_menu)];
    [NSApp setHelpMenu:help_menu];
    [NSApp setMainMenu:main];
    platform_macos_menus_refresh_recent();
    platform_update_menu(main, &platform_command_values);
}

void platform_macos_menus_set_owner(platform_window *window)
{
    platform_delegate.owner = window;
}

void platform_macos_menus_clear_owner(platform_window *window)
{
    if (platform_delegate.owner == window) platform_delegate.owner = NULL;
}

void platform_macos_menus_refresh_recent(void)
{
    if (platform_recent_menu == nil) return;
    platform_append_recent_items(platform_recent_menu);
    platform_recent_parent_item.enabled = platform_recents_count() > 0U;
}

bool platform_macos_menus_set_option_state(platform_window *window,
                                           platform_viewer_action action, bool enabled)
{
    if (window == NULL || platform_delegate.owner != window) return false;
    NSMenuItem *view = [[NSApp mainMenu] itemWithTitle:@"View"];
    if (view == nil) return false;
    for (NSMenuItem *item in view.submenu.itemArray) {
        if (item.tag == (NSInteger)action) {
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
        ![NSThread isMainThread] || logical_x < 0 || logical_y < 0) return false;
    NSView *view = (__bridge NSView *)platform_window_native_view(window);
    if (view == nil) return false;
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@"Ruler"];
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

void platform_macos_menus_update_state(platform_window *window,
                                       const platform_command_state *state)
{
    if (window == NULL || state == NULL || platform_delegate.owner != window) return;
    platform_command_values = *state;
    platform_update_menu(NSApp.mainMenu, state);
}

void platform_macos_menus_set_full_screen(bool is_full_screen)
{
    if (platform_full_screen_item != nil)
        platform_full_screen_item.title = is_full_screen ? @"Exit Full Screen" : @"Enter Full Screen";
}

@implementation VexelApplicationDelegate
- (void)menuNeedsUpdate:(NSMenu *)menu
{
    if (menu != platform_edit_menu) return;
    platform_update_menu(menu, &platform_command_values);
    for (NSMenuItem *item in menu.itemArray) {
        if (item.tag == PLATFORM_ACTION_PASTE)
            item.enabled = item.enabled && platform_clipboard_can_paste_svg();
    }
}

- (void)dispatchCommand:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    NSMenuItem *item = (NSMenuItem *)sender;
    platform_viewer_action action = (platform_viewer_action)item.tag;
    uint16_t scale = action == PLATFORM_ACTION_UI_SCALE_110 ? 110U :
        action == PLATFORM_ACTION_UI_SCALE_120 ? 120U : 100U;
    if (action == PLATFORM_ACTION_UI_SCALE_100 ||
        action == PLATFORM_ACTION_UI_SCALE_110 ||
        action == PLATFORM_ACTION_UI_SCALE_120) {
        if (!platform_text_ui_set_ui_scale(scale)) return;
        for (NSMenuItem *choice in item.menu.itemArray) {
            if (choice.tag == PLATFORM_ACTION_UI_SCALE_100 ||
                choice.tag == PLATFORM_ACTION_UI_SCALE_110 ||
                choice.tag == PLATFORM_ACTION_UI_SCALE_120) {
                choice.state = choice == item ? NSControlStateValueOn :
                    NSControlStateValueOff;
            }
        }
    }
    platform_window_dispatch_action(self.owner, action);
}

- (void)actionClearGuides:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    platform_window_dispatch_action(self.owner,
        (platform_viewer_action)[(NSMenuItem *)sender tag]);
}

- (void)openDocument:(id)sender
{
    (void)sender;
    if (self.owner != NULL) (void)platform_window_open_panel(self.owner);
}

- (void)openRecent:(id)sender
{
    if (![sender isKindOfClass:[NSMenuItem class]]) return;
    NSString *path = [(NSMenuItem *)sender representedObject];
    if (path == nil || self.owner == NULL) return;
    const char *bytes = path.fileSystemRepresentation;
    size_t length = 0U;
    if (bytes != NULL) while (length < PLATFORM_PATH_CAP && bytes[length] != '\0') length++;
    if (platform_window_dispatch_file_open(self.owner, bytes, length))
        platform_macos_welcome_hide();
}

- (void)clearRecent:(id)sender
{
    (void)sender;
    platform_recents_clear();
}

- (void)showSettings:(id)sender
{
    (void)sender;
    platform_settings_show(self.owner);
}

- (void)showHelp:(id)sender
{
    (void)sender;
    platform_help_show(self.owner);
}

- (void)showFeedback:(id)sender
{
    (void)sender;
    platform_feedback_show();
}

- (void)copyDiagnostics:(id)sender
{
    (void)sender;
    platform_feedback_copy_diagnostics();
}

- (NSMenu *)applicationDockMenu:(NSApplication *)application
{
    (void)application;
    NSMenu *dock = [[NSMenu alloc] initWithTitle:@"Vexel"];
    [dock addItem:platform_menu_item(@"New SVG", @"", 0,
        @selector(dispatchCommand:), PLATFORM_ACTION_NEW_DOCUMENT)];
    [dock addItem:platform_menu_item(@"Open SVG…", @"",
        0, @selector(openDocument:), PLATFORM_ACTION_NONE)];
    platform_add_separator(dock);
    NSMenu *recent = [[NSMenu alloc] initWithTitle:@"Open Recent"];
    platform_append_recent_items(recent);
    [dock addItem:platform_menu_heading(@"Open Recent", recent)];
    NSMenu *windows = [[NSMenu alloc] initWithTitle:@"Open Windows"];
    NSUInteger added = 0U;
    for (NSWindow *window in NSApp.windows) {
        if (window == nil || !window.isVisible || window.title.length == 0U) continue;
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:window.title
            action:@selector(makeKeyAndOrderFront:) keyEquivalent:@""];
        item.target = window;
        [windows addItem:item];
        added++;
    }
    if (added == 0U) {
        NSMenuItem *empty = [[NSMenuItem alloc] initWithTitle:@"No Open Windows"
            action:nil keyEquivalent:@""];
        empty.enabled = NO;
        [windows addItem:empty];
    }
    [dock addItem:platform_menu_heading(@"Open Windows", windows)];
    return dock;
}

- (BOOL)applicationShouldHandleReopen:(NSApplication *)sender hasVisibleWindows:(BOOL)flag
{
    (void)sender;
    if (flag || self.owner == NULL) return NO;
    if (platform_command_values.has_document)
        (void)platform_window_set_visible(self.owner, true);
    else
        platform_macos_welcome_show(self.owner);
    return YES;
}

- (void)application:(NSApplication *)application openFiles:(NSArray<NSString *> *)filenames
{
    (void)application;
    BOOL accepted = NO;
    for (NSString *path in filenames) {
        const char *bytes = path.fileSystemRepresentation;
        size_t length = 0U;
        if (bytes != NULL) while (length < PLATFORM_PATH_CAP && bytes[length] != '\0') length++;
        accepted |= platform_window_dispatch_file_open(self.owner, bytes, length);
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
        size_t length = 0U;
        if (bytes != NULL) while (length < PLATFORM_PATH_CAP && bytes[length] != '\0') length++;
        (void)platform_window_dispatch_file_open(self.owner, bytes, length);
    }
}
@end
