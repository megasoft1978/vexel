#import <Cocoa/Cocoa.h>

#include "platform/platform_settings.h"
#include "platform/macos/platform_window_internal.h"
#include "platform/platform_recents.h"

#include <string.h>

typedef NS_ENUM(NSInteger, VexelSettingTag) {
    VexelSettingShowWelcome = 1,
    VexelSettingRememberWorkspace,
    VexelSettingCheckerboard,
    VexelSettingPixelGrid,
    VexelSettingModifierScrollZooms
};

static NSString *const VexelSettingsVersionKey = @"VexelSettingsVersion";
static NSString *const VexelSettingsPrefix = @"VexelSettings.v1.";

@interface VexelSettingsController : NSObject <NSWindowDelegate>
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) NSSegmentedControl *paneSelector;
@property(nonatomic, strong) NSView *paneHost;
@property(nonatomic, strong) NSButton *showWelcome;
@property(nonatomic, strong) NSButton *rememberWorkspace;
@property(nonatomic, strong) NSButton *checkerboard;
@property(nonatomic, strong) NSButton *pixelGrid;
@property(nonatomic, strong) NSButton *modifierScrollZooms;
@property(nonatomic, assign) platform_window *owner;
@property(nonatomic) NSInteger activePane;
- (void)showForOwner:(platform_window *)owner;
- (void)selectPane:(id)sender;
- (void)settingChanged:(id)sender;
- (void)resetDefaults:(id)sender;
- (void)clearRecentDocuments:(id)sender;
@end

static VexelSettingsController *settings_controller;

static NSUserDefaults *settings_test_defaults;
static NSString *settings_test_domain;
static size_t settings_write_total;

/* All reads and writes go through here so a test can redirect them to a
 * throwaway domain and the user's real preferences are never touched. */
static NSUserDefaults *settings_defaults(void)
{
    return settings_test_defaults != nil ? settings_test_defaults :
        [NSUserDefaults standardUserDefaults];
}

static void settings_note_write(void)
{
    settings_write_total++;
}

void platform_settings_use_test_domain(const char *domain)
{
    if (![NSThread isMainThread]) return;
    if (settings_test_defaults != nil && settings_test_domain != nil) {
        [settings_test_defaults removePersistentDomainForName:settings_test_domain];
        [settings_test_defaults synchronize];
        NSString *plist = [[NSHomeDirectory()
            stringByAppendingPathComponent:@"Library/Preferences"]
            stringByAppendingPathComponent:
                [settings_test_domain stringByAppendingString:@".plist"]];
        [[NSFileManager defaultManager] removeItemAtPath:plist error:nil];
    }
    settings_test_defaults = nil;
    settings_test_domain = nil;
    settings_write_total = 0U;
    if (domain == NULL || domain[0] == '\0') return;
    NSString *name = [NSString stringWithUTF8String:domain];
    if (name == nil || name.length == 0U) return;
    [[NSUserDefaults standardUserDefaults] removePersistentDomainForName:name];
    settings_test_defaults = [[NSUserDefaults alloc] initWithSuiteName:name];
    settings_test_domain = name;
}

size_t platform_settings_write_count(void)
{
    return settings_write_total;
}

static NSString *settings_key(NSString *suffix)
{
    return [VexelSettingsPrefix stringByAppendingString:suffix];
}

static NSString *settings_string(NSString *key, NSString *fallback)
{
    id stored = [settings_defaults() objectForKey:key];
    if (![stored isKindOfClass:[NSString class]]) return fallback;
    return (NSString *)stored;
}

static BOOL settings_bool(NSString *key, BOOL fallback)
{
    id stored = [settings_defaults() objectForKey:key];
    return stored == nil ? fallback : [stored boolValue];
}

static void settings_seed_defaults(void)
{
    NSUserDefaults *defaults = settings_defaults();
    id version = [defaults objectForKey:VexelSettingsVersionKey];
    if ([version respondsToSelector:@selector(integerValue)] &&
        [version integerValue] == 1) return;
    settings_note_write();
    [defaults setInteger:1 forKey:VexelSettingsVersionKey];
    [defaults setBool:YES forKey:settings_key(@"showWelcome")];
    [defaults setBool:YES forKey:settings_key(@"rememberWorkspace")];
    [defaults setBool:YES forKey:settings_key(@"structureVisible")];
    [defaults setBool:YES forKey:settings_key(@"inspectorVisible")];
    [defaults setBool:YES forKey:settings_key(@"checkerboard")];
    [defaults setBool:NO forKey:settings_key(@"pixelGrid")];
    [defaults setBool:YES forKey:settings_key(@"modifierScrollZooms")];
    [defaults setInteger:0 forKey:settings_key(@"lastPane")];
}

void platform_settings_get(platform_settings_values *out_values)
{
    if (out_values == NULL || ![NSThread isMainThread]) return;
    settings_seed_defaults();
    BOOL remember = settings_bool(settings_key(@"rememberWorkspace"), YES);
    *out_values = (platform_settings_values){
        .show_welcome_on_launch = settings_bool(settings_key(@"showWelcome"), YES),
        .remember_workspace = remember,
        .structure_visible = remember ? settings_bool(settings_key(@"structureVisible"), YES) : YES,
        .inspector_visible = remember ? settings_bool(settings_key(@"inspectorVisible"), YES) : YES,
        .default_checkerboard = settings_bool(settings_key(@"checkerboard"), YES),
        .default_pixel_grid = settings_bool(settings_key(@"pixelGrid"), NO),
        .modifier_scroll_zooms = settings_bool(settings_key(@"modifierScrollZooms"), YES),
        .workspace = {0},
        .density = (uint8_t)[settings_defaults()
            integerForKey:settings_key(@"density")]
    };
    if (remember) {
        NSString *workspace = settings_string(settings_key(@"workspace"), @"");
        const char *name = [workspace UTF8String];
        if (name != NULL) {
            size_t length = strlen(name);
            if (length >= sizeof(out_values->workspace))
                length = sizeof(out_values->workspace) - 1U;
            memcpy(out_values->workspace, name, length);
            out_values->workspace[length] = '\0';
        }
    }
}

void platform_settings_store_density(uint8_t density)
{
    if (![NSThread isMainThread]) return;
    settings_seed_defaults();
    NSUserDefaults *defaults = settings_defaults();
    if (!settings_bool(settings_key(@"rememberWorkspace"), YES)) return;
    if ((uint8_t)[defaults integerForKey:settings_key(@"density")] == density) return;
    settings_note_write();
    [defaults setInteger:(NSInteger)density forKey:settings_key(@"density")];
}

void platform_settings_store_workspace(const char *name)
{
    if (![NSThread isMainThread]) return;
    settings_seed_defaults();
    NSUserDefaults *defaults = settings_defaults();
    if (!settings_bool(settings_key(@"rememberWorkspace"), YES)) return;
    NSString *key = settings_key(@"workspace");
    NSString *value = name == NULL ? @"" : [NSString stringWithUTF8String:name];
    NSString *current = settings_string(key, @"");
    if (value != nil && ![current isEqualToString:value]) {
        settings_note_write();
        [defaults setObject:value forKey:key];
    }
}

void platform_settings_store_sidebar_visibility(bool structure_visible,
                                                bool inspector_visible)
{
    if (![NSThread isMainThread]) return;
    settings_seed_defaults();
    NSUserDefaults *defaults = settings_defaults();
    if (!settings_bool(settings_key(@"rememberWorkspace"), YES)) return;
    NSString *structure_key = settings_key(@"structureVisible");
    NSString *inspector_key = settings_key(@"inspectorVisible");
    if ([defaults boolForKey:structure_key] != structure_visible) {
        settings_note_write();
        [defaults setBool:structure_visible forKey:structure_key];
    }
    if ([defaults boolForKey:inspector_key] != inspector_visible) {
        settings_note_write();
        [defaults setBool:inspector_visible forKey:inspector_key];
    }
}

static NSButton *settings_checkbox(NSString *title, VexelSettingTag tag,
                                   id target, SEL action)
{
    NSButton *button = [NSButton checkboxWithTitle:title target:target action:action];
    button.tag = tag;
    button.translatesAutoresizingMaskIntoConstraints = NO;
    return button;
}

static NSStackView *settings_stack(void)
{
    NSStackView *stack = [[NSStackView alloc] initWithFrame:NSZeroRect];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.distribution = NSStackViewDistributionFill;
    stack.spacing = 14.0;
    stack.edgeInsets = NSEdgeInsetsMake(16.0, 20.0, 16.0, 20.0);
    stack.translatesAutoresizingMaskIntoConstraints = NO;
    return stack;
}

@implementation VexelSettingsController
- (instancetype)init
{
    self = [super init];
    if (self == nil) return nil;
    settings_seed_defaults();
    NSRect frame = NSMakeRect(0.0, 0.0, 560.0, 380.0);
    self.window = [[NSWindow alloc] initWithContentRect:frame
        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
        backing:NSBackingStoreBuffered defer:NO];
    self.window.releasedWhenClosed = NO;
    self.window.title = @"Settings";
    self.window.delegate = self;
    NSButton *minimize = [self.window standardWindowButton:NSWindowMiniaturizeButton];
    NSButton *zoom = [self.window standardWindowButton:NSWindowZoomButton];
    minimize.enabled = NO;
    zoom.enabled = NO;

    NSView *root = [[NSView alloc] initWithFrame:frame];
    root.translatesAutoresizingMaskIntoConstraints = NO;
    self.paneSelector = [[NSSegmentedControl alloc] initWithFrame:NSZeroRect];
    self.paneSelector.segmentCount = 2;
    [self.paneSelector setLabel:@"General" forSegment:0];
    [self.paneSelector setLabel:@"Canvas" forSegment:1];
    self.paneSelector.trackingMode = NSSegmentSwitchTrackingSelectOne;
    self.paneSelector.target = self;
    self.paneSelector.action = @selector(selectPane:);
    self.paneSelector.translatesAutoresizingMaskIntoConstraints = NO;
    [root addSubview:self.paneSelector];
    self.paneHost = [[NSView alloc] initWithFrame:NSZeroRect];
    self.paneHost.translatesAutoresizingMaskIntoConstraints = NO;
    [root addSubview:self.paneHost];
    [NSLayoutConstraint activateConstraints:@[
        [self.paneSelector.topAnchor constraintEqualToAnchor:root.topAnchor constant:18.0],
        [self.paneSelector.centerXAnchor constraintEqualToAnchor:root.centerXAnchor],
        [self.paneSelector.widthAnchor constraintEqualToConstant:240.0],
        [self.paneHost.topAnchor constraintEqualToAnchor:self.paneSelector.bottomAnchor constant:12.0],
        [self.paneHost.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [self.paneHost.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],
        [self.paneHost.bottomAnchor constraintEqualToAnchor:root.bottomAnchor]
    ]];
    self.window.contentView = root;

    self.showWelcome = settings_checkbox(@"Show Welcome when no document is open",
        VexelSettingShowWelcome, self, @selector(settingChanged:));
    self.rememberWorkspace = settings_checkbox(@"Remember sidebar visibility",
        VexelSettingRememberWorkspace, self, @selector(settingChanged:));
    self.checkerboard = settings_checkbox(@"Use checkerboard by default",
        VexelSettingCheckerboard, self, @selector(settingChanged:));
    self.pixelGrid = settings_checkbox(@"Show pixel grid by default",
        VexelSettingPixelGrid, self, @selector(settingChanged:));
    self.modifierScrollZooms = settings_checkbox(@"Command or Control scroll zooms the canvas",
        VexelSettingModifierScrollZooms, self, @selector(settingChanged:));
    [self refreshControls];
    self.activePane = (NSInteger)[settings_defaults()
        integerForKey:settings_key(@"lastPane")];
    if (self.activePane < 0 || self.activePane > 1) self.activePane = 0;
    self.paneSelector.selectedSegment = self.activePane;
    [self rebuildPane];
    return self;
}

- (void)showForOwner:(platform_window *)owner
{
    self.owner = owner;
    [self refreshControls];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

- (void)refreshControls
{
    platform_settings_values values = {0};
    platform_settings_get(&values);
    self.showWelcome.state = values.show_welcome_on_launch ? NSControlStateValueOn : NSControlStateValueOff;
    self.rememberWorkspace.state = values.remember_workspace ? NSControlStateValueOn : NSControlStateValueOff;
    self.checkerboard.state = values.default_checkerboard ? NSControlStateValueOn : NSControlStateValueOff;
    self.pixelGrid.state = values.default_pixel_grid ? NSControlStateValueOn : NSControlStateValueOff;
    self.modifierScrollZooms.state = values.modifier_scroll_zooms ? NSControlStateValueOn : NSControlStateValueOff;
}

- (void)rebuildPane
{
    [self.paneHost.subviews makeObjectsPerformSelector:@selector(removeFromSuperview)];
    NSStackView *stack = settings_stack();
    [self.paneHost addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:self.paneHost.topAnchor],
        [stack.leadingAnchor constraintEqualToAnchor:self.paneHost.leadingAnchor],
        [stack.trailingAnchor constraintEqualToAnchor:self.paneHost.trailingAnchor],
        [stack.bottomAnchor constraintLessThanOrEqualToAnchor:self.paneHost.bottomAnchor]
    ]];
    NSTextField *description = [NSTextField labelWithString:self.activePane == 0 ?
        @"General app behavior" : @"New-document canvas defaults"];
    description.font = [NSFont systemFontOfSize:13.0 weight:NSFontWeightSemibold];
    [stack addArrangedSubview:description];
    if (self.activePane == 0) {
        [stack addArrangedSubview:self.showWelcome];
        [stack addArrangedSubview:self.rememberWorkspace];
        NSButton *clear = [NSButton buttonWithTitle:@"Clear Recent Documents"
            target:self action:@selector(clearRecentDocuments:)];
        [stack addArrangedSubview:clear];
    } else {
        [stack addArrangedSubview:self.checkerboard];
        [stack addArrangedSubview:self.pixelGrid];
        [stack addArrangedSubview:self.modifierScrollZooms];
    }
    NSView *spacer = [[NSView alloc] initWithFrame:NSZeroRect];
    spacer.translatesAutoresizingMaskIntoConstraints = NO;
    [stack addArrangedSubview:spacer];
    NSButton *reset = [NSButton buttonWithTitle:@"Restore Defaults"
        target:self action:@selector(resetDefaults:)];
    [stack addArrangedSubview:reset];
}

- (void)selectPane:(id)sender
{
    (void)sender;
    NSInteger pane = self.paneSelector.selectedSegment;
    if (pane < 0 || pane > 1) return;
    self.activePane = pane;
    [settings_defaults() setInteger:pane forKey:settings_key(@"lastPane")];
    [self rebuildPane];
}

- (void)settingChanged:(id)sender
{
    if (![sender isKindOfClass:[NSButton class]]) return;
    NSButton *button = (NSButton *)sender;
    NSUserDefaults *defaults = settings_defaults();
    platform_viewer_action changed_action = PLATFORM_ACTION_NONE;
    switch ((VexelSettingTag)button.tag) {
        case VexelSettingShowWelcome:
            [defaults setBool:button.state == NSControlStateValueOn forKey:settings_key(@"showWelcome")];
            break;
        case VexelSettingRememberWorkspace:
            [defaults setBool:button.state == NSControlStateValueOn forKey:settings_key(@"rememberWorkspace")];
            changed_action = PLATFORM_ACTION_APPLY_WORKSPACE_PREFERENCE;
            break;
        case VexelSettingCheckerboard:
            [defaults setBool:button.state == NSControlStateValueOn forKey:settings_key(@"checkerboard")];
            changed_action = PLATFORM_ACTION_APPLY_CHECKERBOARD_PREFERENCE;
            break;
        case VexelSettingPixelGrid:
            [defaults setBool:button.state == NSControlStateValueOn forKey:settings_key(@"pixelGrid")];
            changed_action = PLATFORM_ACTION_APPLY_PIXEL_GRID_PREFERENCE;
            break;
        case VexelSettingModifierScrollZooms:
            [defaults setBool:button.state == NSControlStateValueOn forKey:settings_key(@"modifierScrollZooms")];
            changed_action = PLATFORM_ACTION_APPLY_NAVIGATION_PREFERENCE;
            break;
        default: return;
    }
    if (changed_action != PLATFORM_ACTION_NONE)
        platform_window_dispatch_action(self.owner, changed_action);
}

- (void)resetDefaults:(id)sender
{
    (void)sender;
    NSUserDefaults *defaults = settings_defaults();
    [defaults removeObjectForKey:VexelSettingsVersionKey];
    for (NSString *suffix in @[@"showWelcome", @"rememberWorkspace", @"structureVisible",
            @"inspectorVisible", @"checkerboard", @"pixelGrid", @"modifierScrollZooms"])
        [defaults removeObjectForKey:settings_key(suffix)];
    settings_seed_defaults();
    [self refreshControls];
    self.activePane = 0;
    self.paneSelector.selectedSegment = 0;
    [self rebuildPane];
    platform_window_dispatch_action(self.owner, PLATFORM_ACTION_REFRESH_PREFERENCES);
}

- (void)clearRecentDocuments:(id)sender
{
    (void)sender;
    platform_recents_clear();
}

- (void)windowWillClose:(NSNotification *)notification
{
    (void)notification;
    self.owner = NULL;
}
@end

void platform_settings_show(platform_window *owner_window)
{
    if (![NSThread isMainThread]) return;
    if (settings_controller == nil)
        settings_controller = [[VexelSettingsController alloc] init];
    [settings_controller showForOwner:owner_window];
}

#if defined(VEXEL_NATIVE_UI_GOLDEN_TEST)
NSView *platform_settings_golden_view(NSInteger pane)
{
    static VexelSettingsController *controller;
    if (controller == nil) controller = [[VexelSettingsController alloc] init];
    controller.activePane = pane == 1 ? 1 : 0;
    [controller.paneSelector setSelectedSegment:controller.activePane];
    [controller rebuildPane];
    return controller.window.contentView;
}
#endif
