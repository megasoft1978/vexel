#import <Cocoa/Cocoa.h>

#include "platform_window_internal.h"

#include <string.h>

#define PLATFORM_RECENT_LIMIT 12U

@interface VexelApplicationDelegate : NSObject <NSApplicationDelegate>
@property(nonatomic, assign) platform_window *owner;
- (void)actionOpenDocument:(id)sender;
- (void)actionReload:(id)sender;
- (void)actionToggleCheckerboard:(id)sender;
- (void)actionCycleBackground:(id)sender;
- (void)actionFitWidth:(id)sender;
- (void)actionFitWindow:(id)sender;
- (void)actionActualSize:(id)sender;
- (void)actionTogglePixelGrid:(id)sender;
- (void)actionQuit:(id)sender;
- (void)actionOpenRecent:(id)sender;
- (void)actionClearRecent:(id)sender;
@end

static __strong VexelApplicationDelegate *platform_delegate;
static NSMenu *platform_recent_menu;

static NSMenuItem *platform_menu_item(NSString *title, NSString *key, SEL action,
                                      NSInteger tag)
{
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:action keyEquivalent:key];
    item.target = platform_delegate;
    item.tag = tag;
    item.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    return item;
}

static bool path_is_svg(const char *path, size_t length)
{
    if (path == NULL || length == 0U || length >= PLATFORM_PATH_CAP ||
        memchr(path, '\0', length) != NULL) return false;
    NSString *value = [[NSString alloc] initWithBytes:path length:length
                                             encoding:NSUTF8StringEncoding];
    return value != nil && [[value.pathExtension lowercaseString] isEqualToString:@"svg"];
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
    [application_menu addItem:platform_menu_item(@"Quit vexel", @"q",
        @selector(actionQuit:), PLATFORM_ACTION_QUIT)];
    application.submenu = application_menu;
    [main addItem:application];

    NSMenuItem *file = [[NSMenuItem alloc] initWithTitle:@"File" action:nil keyEquivalent:@""];
    NSMenu *file_menu = [[NSMenu alloc] initWithTitle:@"File"];
    [file_menu addItem:platform_menu_item(@"Open…", @"o", @selector(actionOpenDocument:), 0)];
    [file_menu addItem:platform_menu_item(@"Reload", @"r",
        @selector(actionReload:), PLATFORM_ACTION_RELOAD)];
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

    NSMenuItem *view = [[NSMenuItem alloc] initWithTitle:@"View" action:nil keyEquivalent:@""];
    NSMenu *view_menu = [[NSMenu alloc] initWithTitle:@"View"];
    NSMenuItem *checker = platform_menu_item(@"Checkerboard", @"t",
        @selector(actionToggleCheckerboard:), PLATFORM_ACTION_TOGGLE_CHECKERBOARD);
    checker.state = NSControlStateValueOn;
    [view_menu addItem:checker];
    [view_menu addItem:platform_menu_item(@"Cycle Background", @"b",
        @selector(actionCycleBackground:), PLATFORM_ACTION_CYCLE_BACKGROUND)];
    [view_menu addItem:[NSMenuItem separatorItem]];
    [view_menu addItem:platform_menu_item(@"Fit Width", @"",
        @selector(actionFitWidth:), PLATFORM_ACTION_FIT_WIDTH)];
    [view_menu addItem:platform_menu_item(@"Fit in Window", @"0",
        @selector(actionFitWindow:), PLATFORM_ACTION_FIT_WINDOW)];
    [view_menu addItem:platform_menu_item(@"Actual Size", @"1",
        @selector(actionActualSize:), PLATFORM_ACTION_ACTUAL_SIZE)];
    [view_menu addItem:platform_menu_item(@"Pixel Grid", @"g",
        @selector(actionTogglePixelGrid:), PLATFORM_ACTION_TOGGLE_PIXEL_GRID)];
    view.submenu = view_menu;
    [main addItem:view];

    NSMenuItem *window_item = [[NSMenuItem alloc] initWithTitle:@"Window" action:nil keyEquivalent:@""];
    NSMenu *window_menu = [[NSMenu alloc] initWithTitle:@"Window"];
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

bool platform_macos_menus_set_option_state(platform_window *window,
                                           platform_viewer_action action, bool enabled)
{
    if (window == NULL || platform_delegate.owner != window) return false;
    NSMenuItem *view = [[NSApp mainMenu] itemWithTitle:@"View"];
    for (NSMenuItem *item in view.submenu.itemArray) {
        if (item.tag == (NSInteger)action) {
            item.state = enabled ? NSControlStateValueOn : NSControlStateValueOff;
            return true;
        }
    }
    return false;
}

@implementation VexelApplicationDelegate
- (void)actionOpenDocument:(id)sender
{
    (void)sender;
    if (self.owner != NULL) (void)platform_window_open_panel(self.owner);
}
- (void)actionReload:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_RELOAD); }
- (void)actionToggleCheckerboard:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_CHECKERBOARD); }
- (void)actionCycleBackground:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_CYCLE_BACKGROUND); }
- (void)actionFitWidth:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_FIT_WIDTH); }
- (void)actionFitWindow:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_FIT_WINDOW); }
- (void)actionActualSize:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_ACTUAL_SIZE); }
- (void)actionTogglePixelGrid:(id)sender { (void)sender; platform_window_dispatch_action(self.owner, PLATFORM_ACTION_TOGGLE_PIXEL_GRID); }
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
        size_t length = bytes == NULL ? 0U : strnlen(bytes, PLATFORM_PATH_CAP);
        (void)platform_window_dispatch_file_open(self.owner, bytes, length);
    }
}
@end
