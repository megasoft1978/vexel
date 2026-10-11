#import <Cocoa/Cocoa.h>

#include "platform_accessibility.h"
#include "platform/platform_localization.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

@interface VexelSemanticElement : NSAccessibilityElement
@property(nonatomic, copy) NSString *roleName;
@property(nonatomic, copy) NSString *labelText;
@property(nonatomic, copy) NSString *valueText;
@property(nonatomic, copy) NSString *helpText;
@property(nonatomic, copy) NSString *stableIdentifier;
@property(nonatomic, assign) NSRect screenFrame;
@property(nonatomic, strong) id parentElement;
@property(nonatomic, copy) NSArray *childElements;
@property(nonatomic, assign) BOOL isEnabled;
@property(nonatomic, assign) BOOL isSelected;
@property(nonatomic, assign) BOOL isFocused;
@property(nonatomic, assign) BOOL isCheckable;
@property(nonatomic, assign) BOOL isChecked;
@property(nonatomic, assign) BOOL isExpandable;
@property(nonatomic, assign) BOOL isExpanded;
@property(nonatomic, assign) uint32_t commandIdentifier;
@property(nonatomic, assign) platform_accessibility_command_callback commandCallback;
@property(nonatomic, assign) void *commandContext;
- (BOOL)performPointerPress;
@end

@implementation VexelSemanticElement
- (NSAccessibilityRole)accessibilityRole { return self.roleName; }
- (NSString *)accessibilityLabel { return self.labelText; }
- (id)accessibilityValue
{
    if (self.isCheckable) return @(self.isChecked);
    return self.valueText.length == 0U ? nil : self.valueText;
}
- (NSString *)accessibilityHelp { return self.helpText; }
- (NSString *)accessibilityIdentifier { return self.stableIdentifier; }
- (NSRect)accessibilityFrame { return self.screenFrame; }
- (id)accessibilityParent { return self.parentElement; }
- (NSArray *)accessibilityChildren { return self.childElements; }
- (NSArray<id<NSAccessibilityElement>> *)accessibilityChildrenInNavigationOrder
{
    return self.childElements;
}
- (BOOL)isAccessibilityEnabled { return self.isEnabled; }
- (BOOL)isAccessibilitySelected { return self.isSelected; }
- (BOOL)isAccessibilityFocused { return self.isFocused; }
- (BOOL)isAccessibilityExpanded { return self.isExpandable && self.isExpanded; }
- (BOOL)isAccessibilityElement { return YES; }
- (BOOL)accessibilityPerformPress
{
    if (!self.isEnabled || self.commandIdentifier == 0U) return NO;
    if (self.isCheckable && self.commandCallback != NULL)
        return self.commandCallback(self.commandIdentifier, self.commandContext);
    if ([self performPointerPress]) return YES;
    return self.commandCallback != NULL &&
        self.commandCallback(self.commandIdentifier, self.commandContext);
}
- (BOOL)performPointerPress
{
    NSWindow *window = NSApp.keyWindow;
    if (window == nil || window.contentView == nil || self.screenFrame.size.width <= 0.0 ||
        self.screenFrame.size.height <= 0.0 || ![NSThread isMainThread]) return NO;
    NSPoint screen = NSMakePoint(NSMidX(self.screenFrame), NSMidY(self.screenFrame));
    NSPoint location = [window convertPointFromScreen:screen];
    NSTimeInterval timestamp = NSProcessInfo.processInfo.systemUptime;
    NSEvent *down = [NSEvent mouseEventWithType:NSEventTypeLeftMouseDown
        location:location modifierFlags:0 timestamp:timestamp
        windowNumber:window.windowNumber context:nil eventNumber:0 clickCount:1 pressure:1.0];
    NSEvent *up = [NSEvent mouseEventWithType:NSEventTypeLeftMouseUp
        location:location modifierFlags:0 timestamp:timestamp + 0.001
        windowNumber:window.windowNumber context:nil eventNumber:0 clickCount:1 pressure:0.0];
    if (down == nil || up == nil) return NO;
    [NSApp sendEvent:down];
    [NSApp sendEvent:up];
    return YES;
}
@end

@interface VexelViewMenuActions : NSObject
@property(nonatomic, assign) platform_accessibility_view_callback callback;
@property(nonatomic, assign) void *context;
@property(nonatomic, assign) BOOL structureVisible;
@property(nonatomic, assign) BOOL inspectorVisible;
@property(nonatomic, strong) NSMenuItem *structureItem;
@property(nonatomic, strong) NSMenuItem *inspectorItem;
@property(nonatomic, strong) VexelSemanticElement *structureControl;
@property(nonatomic, strong) VexelSemanticElement *inspectorControl;
- (void)toggleStructure:(id)sender;
- (void)toggleInspector:(id)sender;
@end

static VexelViewMenuActions *view_menu_actions;
static platform_accessibility_command_callback command_callback;
static void *command_context;
static NSMutableDictionary<NSNumber *, NSString *> *last_status_announcements;
static ui_semantics_snapshot current_semantics_snapshot;

static NSString *string_from_slice(const ui_semantics_snapshot *snapshot,
                                   const ui_semantics_node *node, bool value)
{
    const char *bytes = NULL;
    size_t length = 0U;
    if (!ui_semantics_text(snapshot, node, value, &bytes, &length) ||
        length > 64U * 1024U) return @"";
    NSString *text = [[NSString alloc] initWithBytes:bytes length:length
        encoding:NSUTF8StringEncoding];
    return text == nil ? @"" : text;
}

static NSAccessibilityRole role_for_ui_role(ui_role role)
{
    switch (role) {
        case UI_ROLE_WINDOW: return NSAccessibilityGroupRole;
        case UI_ROLE_BUTTON: return NSAccessibilityButtonRole;
        case UI_ROLE_CHECKBOX: return NSAccessibilityCheckBoxRole;
        case UI_ROLE_TEXT_FIELD: return NSAccessibilityTextFieldRole;
        case UI_ROLE_SLIDER: return NSAccessibilitySliderRole;
        case UI_ROLE_LIST: return NSAccessibilityListRole;
        case UI_ROLE_ROW: return NSAccessibilityRowRole;
        case UI_ROLE_STATUS: return NSAccessibilityStaticTextRole;
        case UI_ROLE_TEXT: return NSAccessibilityStaticTextRole;
        case UI_ROLE_GROUP: return NSAccessibilityGroupRole;
        case UI_ROLE_NONE: default: return NSAccessibilityUnknownRole;
    }
}

static VexelSemanticElement *element_for_id(NSDictionary<NSNumber *,
    VexelSemanticElement *> *elements, uint32_t identifier)
{
    return elements[@(identifier)];
}

static void install_view_menu(void)
{
    if (NSApp == nil || view_menu_actions == nil) return;
    NSString *view_title = platform_localization_string(@"menu.view.title", @"View",
        @"View menu title.");
    NSMenuItem *view = [[NSApp mainMenu] itemWithTitle:view_title];
    NSMenu *menu = view.submenu;
    if (menu == nil) return;
    if (view_menu_actions.structureItem == nil) {
        NSString *title = platform_localization_string(@"menu.view.show_layers",
            @"Show Layers", @"Show Layers panel menu item.");
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title
            action:@selector(toggleStructure:) keyEquivalent:@""];
        item.target = view_menu_actions;
        item.tag = 0x565301;
        [menu addItem:[NSMenuItem separatorItem]];
        [menu addItem:item];
        view_menu_actions.structureItem = item;
    }
    if (view_menu_actions.inspectorItem == nil) {
        NSString *title = platform_localization_string(@"menu.view.show_inspector",
            @"Show Inspector", @"Show Inspector panel menu item.");
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title
            action:@selector(toggleInspector:) keyEquivalent:@""];
        item.target = view_menu_actions;
        item.tag = 0x565302;
        [menu addItem:item];
        view_menu_actions.inspectorItem = item;
    }
    view_menu_actions.structureItem.state = view_menu_actions.structureVisible ?
        NSControlStateValueOn : NSControlStateValueOff;
    view_menu_actions.inspectorItem.state = view_menu_actions.inspectorVisible ?
        NSControlStateValueOn : NSControlStateValueOff;
}

static void post_status_announcement(uint32_t identifier, NSString *status,
                                     BOOL isError)
{
    if (status.length == 0U) return;
    if (last_status_announcements == nil)
        last_status_announcements = [NSMutableDictionary dictionary];
    NSNumber *key = @(identifier);
    NSString *previous = last_status_announcements[key];
    if (previous != nil && ![status isEqualToString:previous]) {
        NSDictionary *info = @{NSAccessibilityAnnouncementKey: status,
            NSAccessibilityPriorityKey: @(isError ? NSAccessibilityPriorityHigh :
                NSAccessibilityPriorityMedium)};
        NSAccessibilityPostNotificationWithUserInfo(NSApp,
            NSAccessibilityAnnouncementRequestedNotification, info);
    }
    last_status_announcements[key] = [status copy];
}

static VexelSemanticElement *layer_action_element(const ui_semantics_node *row,
    NSString *row_label, VexelSemanticElement *parent, BOOL visibility, BOOL active,
    uint32_t pixel_height, double backing_scale)
{
    VexelSemanticElement *element = [[VexelSemanticElement alloc] init];
    NSString *action = visibility ? platform_localization_string(
        @"editor.accessibility.layer_visibility", @"Layer visibility",
        @"VoiceOver layer visibility control label.") :
        platform_localization_string(@"editor.accessibility.layer_lock", @"Layer lock",
        @"VoiceOver layer lock control label.");
    NSString *help;
    if (visibility) {
        help = platform_localization_string(active ? @"editor.accessibility.press_hide" :
            @"editor.accessibility.press_show", active ? @"Press to hide this layer." :
            @"Press to show this layer.", @"Help for the layer visibility control.");
    } else {
        help = platform_localization_string(active ? @"editor.accessibility.press_unlock" :
            @"editor.accessibility.press_lock", active ? @"Press to unlock this layer." :
            @"Press to lock this layer.", @"Help for the layer lock control.");
    }
    element.roleName = NSAccessibilityCheckBoxRole;
    element.labelText = [NSString stringWithFormat:@"%@, %@", row_label, action];
    element.helpText = help;
    element.stableIdentifier = [NSString stringWithFormat:@"vexel-layer-%u-%@",
        row->id, visibility ? @"visibility" : @"lock"];
    element.isEnabled = YES;
    element.isCheckable = YES;
    element.isChecked = active;
    element.isSelected = active;
    element.parentElement = parent;
    uint32_t row_id = row->command_id != 0U ? row->command_id : row->id;
    element.commandIdentifier = row_id | (visibility ?
        PLATFORM_ACCESSIBILITY_COMMAND_LAYER_VISIBILITY :
        PLATFORM_ACCESSIBILITY_COMMAND_LAYER_LOCK);
    element.commandCallback = command_callback;
    element.commandContext = command_context;
    double scale = backing_scale;
    NSRect view_rect = NSMakeRect((double)row->frame.x / scale,
        ((double)pixel_height - (double)row->frame.y - row->frame.height) / scale,
        (double)row->frame.width / scale, (double)row->frame.height / scale);
    NSWindow *window = NSApp.keyWindow;
    NSRect window_rect = [window.contentView convertRect:view_rect toView:nil];
    NSRect screen_rect = [window convertRectToScreen:window_rect];
    CGFloat width = 16.0;
    CGFloat height = MIN(24.0, NSHeight(screen_rect));
    CGFloat inset = visibility ? 34.0 : 18.0;
    element.screenFrame = NSMakeRect(NSMaxX(screen_rect) - inset,
        NSMidY(screen_rect) - height / 2.0, width, height);
    return element;
}

bool platform_accessibility_update(const ui_semantics_snapshot *snapshot,
    uint32_t pixel_width, uint32_t pixel_height, double backing_scale)
{
    if (snapshot == NULL || snapshot->count == 0U || snapshot->count > UI_MAX_NODES ||
        pixel_width == 0U || pixel_height == 0U || !isfinite(backing_scale) ||
        backing_scale <= 0.0 || NSApp == nil || ![NSThread isMainThread]) return false;
    NSWindow *window = NSApp.keyWindow;
    NSView *content = window.contentView;
    if (content == nil) return false;
    NSMutableDictionary<NSNumber *, VexelSemanticElement *> *elements =
        [NSMutableDictionary dictionaryWithCapacity:snapshot->count];
    NSMutableArray<VexelSemanticElement *> *ordered =
        [NSMutableArray arrayWithCapacity:snapshot->count];
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *source = &snapshot->nodes[index];
        NSString *label = string_from_slice(snapshot, source, false);
        NSString *value = string_from_slice(snapshot, source, true);
        VexelSemanticElement *element = [[VexelSemanticElement alloc] init];
        element.roleName = role_for_ui_role(source->role);
        element.labelText = label;
        element.valueText = value;
        element.stableIdentifier = [NSString stringWithFormat:@"vexel-ui-%u", source->id];
        element.isEnabled = (source->state & UI_STATE_DISABLED) == 0U;
        element.isSelected = (source->state &
            (UI_STATE_CHECKED | UI_STATE_SELECTED)) != 0U;
        element.isFocused = (source->state & UI_STATE_FOCUSED) != 0U;
        element.isCheckable = source->role == UI_ROLE_CHECKBOX;
        element.isChecked = element.isSelected;
        element.isExpandable = (source->state & UI_STATE_HAS_CHILDREN) != 0U;
        element.isExpanded = (source->state & UI_STATE_EXPANDED) != 0U;
        element.commandIdentifier = source->command_id;
        element.commandCallback = command_callback;
        element.commandContext = command_context;
        double scale = backing_scale;
        NSRect view_rect = NSMakeRect((double)source->frame.x / scale,
            ((double)pixel_height - (double)source->frame.y -
                (double)source->frame.height) / scale,
            (double)source->frame.width / scale,
            (double)source->frame.height / scale);
        NSRect window_rect = [content convertRect:view_rect toView:nil];
        element.screenFrame = [window convertRectToScreen:window_rect];
        elements[@(source->id)] = element;
        [ordered addObject:element];
        if (source->role == UI_ROLE_STATUS) {
            if ((source->state & UI_STATE_ANNOUNCEMENT) != 0U)
                post_status_announcement(source->id, label,
                    (source->state & UI_STATE_INVALID) != 0U);
            else {
                if (last_status_announcements == nil)
                    last_status_announcements = [NSMutableDictionary dictionary];
                last_status_announcements[@(source->id)] = [label copy];
            }
        }
        if (source->role == UI_ROLE_BUTTON &&
            [label isEqualToString:platform_localization_string(
                @"editor.panel.layers", @"Layers panel", @"Layers panel toggle label.")])
            view_menu_actions.structureControl = element;
        if (source->role == UI_ROLE_BUTTON &&
            [label isEqualToString:platform_localization_string(
                @"editor.panel.inspector", @"Inspector panel",
                @"Inspector panel toggle label.")])
            view_menu_actions.inspectorControl = element;
    }
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *source = &snapshot->nodes[index];
        VexelSemanticElement *child = ordered[index];
        VexelSemanticElement *parent = source->parent_id == 0U ? nil :
            element_for_id(elements, source->parent_id);
        child.parentElement = parent == nil ? content : parent;
        if (parent != nil) {
            NSMutableArray *children = [parent.childElements mutableCopy];
            if (children == nil) children = [NSMutableArray array];
            [children addObject:child];
            parent.childElements = children;
        }
    }
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *source = &snapshot->nodes[index];
        if (source->role != UI_ROLE_ROW || source->id < 1000U || source->id >= 1020U)
            continue;
        VexelSemanticElement *row = ordered[index];
        NSString *row_label = string_from_slice(snapshot, source, false);
        BOOL visible = (source->state & UI_STATE_HIDDEN) == 0U;
        BOOL locked = (source->state & UI_STATE_LOCKED) != 0U;
        VexelSemanticElement *visibility = layer_action_element(source,
            row_label, row, YES, visible, pixel_height, backing_scale);
        VexelSemanticElement *lock = layer_action_element(source, row_label,
            row, NO, locked, pixel_height, backing_scale);
        NSMutableArray *children = [row.childElements mutableCopy];
        if (children == nil) children = [NSMutableArray array];
        [children addObject:visibility];
        [children addObject:lock];
        row.childElements = children;
    }
    VexelSemanticElement *root = ordered[0];
    root.parentElement = content;
    content.accessibilityChildren = @[root];
    content.accessibilityChildrenInNavigationOrder = @[root];
    install_view_menu();
    NSAccessibilityPostNotification(content, NSAccessibilityLayoutChangedNotification);
    return true;
}

bool platform_accessibility_update_tree(const ui_tree *tree,
    uint32_t pixel_width, uint32_t pixel_height, double backing_scale)
{
    if (tree == NULL || NSApp == nil || ![NSThread isMainThread] ||
        !ui_semantics_snapshot_build(tree, &current_semantics_snapshot)) return false;
    return platform_accessibility_update(&current_semantics_snapshot, pixel_width,
        pixel_height, backing_scale);
}

void platform_accessibility_set_view_callback(
    platform_accessibility_view_callback callback, void *context,
    bool structure_visible, bool inspector_visible)
{
    if (![NSThread isMainThread]) return;
    if (view_menu_actions == nil) view_menu_actions = [[VexelViewMenuActions alloc] init];
    view_menu_actions.callback = callback;
    view_menu_actions.context = context;
    view_menu_actions.structureVisible = structure_visible;
    view_menu_actions.inspectorVisible = inspector_visible;
    install_view_menu();
}

void platform_accessibility_clear_view_callback(void *context)
{
    if (![NSThread isMainThread] || view_menu_actions.context != context) return;
    view_menu_actions.callback = NULL;
    view_menu_actions.context = NULL;
}

void platform_accessibility_set_command_callback(
    platform_accessibility_command_callback callback, void *context)
{
    if (![NSThread isMainThread]) return;
    command_callback = callback;
    command_context = context;
}

@implementation VexelViewMenuActions
- (void)toggleStructure:(id)sender
{
    (void)sender;
    self.structureVisible = !self.structureVisible;
    BOOL sent = [self.structureControl performPointerPress];
    if (!sent && self.callback != NULL)
        self.callback(self.structureVisible, self.inspectorVisible, self.context);
    self.structureItem.state = self.structureVisible ?
        NSControlStateValueOn : NSControlStateValueOff;
    if (NSApp.keyWindow.contentView != nil)
        NSAccessibilityPostNotification(NSApp.keyWindow.contentView,
            NSAccessibilityLayoutChangedNotification);
}
- (void)toggleInspector:(id)sender
{
    (void)sender;
    self.inspectorVisible = !self.inspectorVisible;
    BOOL sent = [self.inspectorControl performPointerPress];
    if (!sent && self.callback != NULL)
        self.callback(self.structureVisible, self.inspectorVisible, self.context);
    self.inspectorItem.state = self.inspectorVisible ?
        NSControlStateValueOn : NSControlStateValueOff;
    if (NSApp.keyWindow.contentView != nil)
        NSAccessibilityPostNotification(NSApp.keyWindow.contentView,
            NSAccessibilityLayoutChangedNotification);
}
@end
