#import <Cocoa/Cocoa.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

#include "platform/platform_recents.h"
#include "platform/macos/platform_window_internal.h"

#include <string.h>

@class VexelWelcomeController;

@interface VexelWelcomeButton : NSButton
@property(nonatomic, copy) NSString *path;
@end

@implementation VexelWelcomeButton
@end

@interface VexelRecentStack : NSStackView
@end

@implementation VexelRecentStack
- (BOOL)isFlipped { return YES; }
@end

@interface VexelWelcomeDropView : NSView
@property(nonatomic, assign) platform_window *owner;
@property(nonatomic, getter=isHighlighted) BOOL highlighted;
@end

@interface VexelWelcomeController : NSObject <NSWindowDelegate>
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) NSStackView *recentStack;
@property(nonatomic, strong) NSScrollView *recentScroll;
@property(nonatomic, strong) NSTextField *emptyRecentLabel;
@property(nonatomic, strong) VexelWelcomeDropView *dropView;
@property(nonatomic, assign) platform_window *owner;
- (void)showForOwner:(platform_window *)owner;
- (void)refresh;
- (void)openPanel:(id)sender;
- (void)newDocument:(id)sender;
- (void)openRecent:(id)sender;
- (void)removeRecent:(id)sender;
- (void)clearRecent:(id)sender;
@end

static VexelWelcomeController *welcome_controller;

static size_t welcome_bounded_length(const char *text, size_t maximum)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= maximum && text[length] != '\0') length++;
    return length;
}

static bool welcome_url_is_svg(NSURL *url)
{
    if (url == nil || !url.isFileURL) return false;
    const char *path = url.path.fileSystemRepresentation;
    size_t length = welcome_bounded_length(path, WELCOME_RECENT_PATH_CAP - 1U);
    return welcome_recent_path_supported(path, length);
}

@implementation VexelWelcomeDropView
- (instancetype)initWithFrame:(NSRect)frame
{
    self = [super initWithFrame:frame];
    if (self != nil) [self registerForDraggedTypes:@[NSPasteboardTypeFileURL]];
    return self;
}
- (BOOL)isFlipped { return YES; }
- (void)drawRect:(NSRect)dirtyRect
{
    (void)dirtyRect;
    NSRect rect = NSInsetRect(self.bounds, 1.0, 1.0);
    NSBezierPath *border = [NSBezierPath bezierPathWithRoundedRect:rect xRadius:9.0 yRadius:9.0];
    CGFloat dash[] = {5.0, 4.0};
    [border setLineDash:dash count:2 phase:0.0];
    border.lineWidth = 1.5;
    NSColor *stroke = self.isHighlighted ? [NSColor controlAccentColor] :
        [NSColor separatorColor];
    [stroke setStroke];
    [border stroke];
    if ([self isHighlighted]) {
        NSColor *fill = [[NSColor controlAccentColor] colorWithAlphaComponent:0.07];
        [fill setFill];
        [border fill];
    }
}
- (NSDragOperation)draggingEntered:(id<NSDraggingInfo>)sender
{
    NSArray<NSURL *> *urls = [sender.draggingPasteboard readObjectsForClasses:@[[NSURL class]]
        options:@{NSPasteboardURLReadingFileURLsOnlyKey: @YES}];
    for (NSURL *url in urls) {
        if (welcome_url_is_svg(url)) {
            self.highlighted = YES;
            [self setNeedsDisplay:YES];
            return NSDragOperationCopy;
        }
    }
    return NSDragOperationNone;
}
- (void)draggingExited:(id<NSDraggingInfo>)sender
{
    (void)sender;
    self.highlighted = NO;
    [self setNeedsDisplay:YES];
}
- (BOOL)performDragOperation:(id<NSDraggingInfo>)sender
{
    self.highlighted = NO;
    [self setNeedsDisplay:YES];
    NSArray<NSURL *> *urls = [sender.draggingPasteboard readObjectsForClasses:@[[NSURL class]]
        options:@{NSPasteboardURLReadingFileURLsOnlyKey: @YES}];
    for (NSURL *url in urls) {
        if (!welcome_url_is_svg(url)) continue;
        const char *path = url.path.fileSystemRepresentation;
        size_t length = welcome_bounded_length(path, WELCOME_RECENT_PATH_CAP - 1U);
        if (platform_window_dispatch_file_open(self.owner, path, length)) return YES;
    }
    return NO;
}
@end

static NSTextField *welcome_label(NSString *text, CGFloat size, NSFontWeight weight)
{
    NSTextField *field = [NSTextField labelWithString:text];
    field.font = [NSFont systemFontOfSize:size weight:weight];
    field.translatesAutoresizingMaskIntoConstraints = NO;
    return field;
}

static NSStackView *welcome_stack(NSUserInterfaceLayoutOrientation orientation,
                                  CGFloat spacing)
{
    NSStackView *stack = [[NSStackView alloc] initWithFrame:NSZeroRect];
    stack.orientation = orientation;
    stack.alignment = orientation == NSUserInterfaceLayoutOrientationHorizontal ?
        NSLayoutAttributeCenterY : NSLayoutAttributeLeading;
    stack.distribution = NSStackViewDistributionFill;
    stack.spacing = spacing;
    stack.translatesAutoresizingMaskIntoConstraints = NO;
    return stack;
}

static VexelWelcomeButton *welcome_path_button(NSString *title,
    VexelWelcomeController *target, SEL action)
{
    VexelWelcomeButton *button = [[VexelWelcomeButton alloc] initWithFrame:NSZeroRect];
    button.title = title;
    button.target = target;
    button.action = action;
    button.bezelStyle = NSBezelStyleRounded;
    return button;
}

@implementation VexelWelcomeController
- (instancetype)init
{
    self = [super init];
    if (self == nil) return nil;
    NSRect frame = NSMakeRect(0.0, 0.0, 600.0, 650.0);
    self.window = [[NSWindow alloc] initWithContentRect:frame
        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
        backing:NSBackingStoreBuffered defer:NO];
    self.window.releasedWhenClosed = NO;
    self.window.title = @"Welcome to Vexel";
    self.window.delegate = self;
    NSView *root = [[NSView alloc] initWithFrame:frame];
    root.translatesAutoresizingMaskIntoConstraints = NO;
    self.window.contentView = root;

    NSStackView *content = welcome_stack(NSUserInterfaceLayoutOrientationVertical, 12.0);
    content.edgeInsets = NSEdgeInsetsMake(24.0, 28.0, 24.0, 28.0);
    [root addSubview:content];
    [NSLayoutConstraint activateConstraints:@[
        [content.topAnchor constraintEqualToAnchor:root.topAnchor],
        [content.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [content.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],
        [content.bottomAnchor constraintLessThanOrEqualToAnchor:root.bottomAnchor]
    ]];
    [content addArrangedSubview:welcome_label(@"Welcome to Vexel", 28.0, NSFontWeightBold)];
    [content addArrangedSubview:welcome_label(@"Open an SVG, start a blank document, or continue your recent work.",
        13.0, NSFontWeightRegular)];

    NSStackView *actions = welcome_stack(NSUserInterfaceLayoutOrientationHorizontal, 10.0);
    NSButton *open = [NSButton buttonWithTitle:@"Open SVG…" target:self action:@selector(openPanel:)];
    NSButton *create = [NSButton buttonWithTitle:@"New Blank SVG" target:self action:@selector(newDocument:)];
    open.bezelStyle = NSBezelStyleRounded;
    create.bezelStyle = NSBezelStyleRounded;
    open.keyEquivalent = @"o";
    open.keyEquivalentModifierMask = NSEventModifierFlagCommand;
    [actions addArrangedSubview:open];
    [actions addArrangedSubview:create];
    [content addArrangedSubview:actions];

    self.dropView = [[VexelWelcomeDropView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 540.0, 58.0)];
    self.dropView.translatesAutoresizingMaskIntoConstraints = NO;
    NSTextField *dropLabel = [NSTextField labelWithString:@"Drag an SVG file here"];
    dropLabel.alignment = NSTextAlignmentCenter;
    dropLabel.textColor = NSColor.secondaryLabelColor;
    dropLabel.translatesAutoresizingMaskIntoConstraints = NO;
    [self.dropView addSubview:dropLabel];
    [NSLayoutConstraint activateConstraints:@[
        [self.dropView.heightAnchor constraintEqualToConstant:58.0],
        [dropLabel.centerXAnchor constraintEqualToAnchor:self.dropView.centerXAnchor],
        [dropLabel.centerYAnchor constraintEqualToAnchor:self.dropView.centerYAnchor]
    ]];
    [content addArrangedSubview:self.dropView];

    NSStackView *recentHeader = welcome_stack(NSUserInterfaceLayoutOrientationHorizontal, 10.0);
    [recentHeader addArrangedSubview:welcome_label(@"Recent Documents", 16.0, NSFontWeightSemibold)];
    NSButton *clear = [NSButton buttonWithTitle:@"Clear Recent Documents"
        target:self action:@selector(clearRecent:)];
    clear.bezelStyle = NSBezelStyleInline;
    [recentHeader addArrangedSubview:clear];
    [content addArrangedSubview:recentHeader];

    self.recentScroll = [[NSScrollView alloc] initWithFrame:NSZeroRect];
    self.recentScroll.translatesAutoresizingMaskIntoConstraints = NO;
    self.recentScroll.hasVerticalScroller = YES;
    self.recentScroll.drawsBackground = NO;
    self.recentStack = [[VexelRecentStack alloc] initWithFrame:NSZeroRect];
    self.recentStack.orientation = NSUserInterfaceLayoutOrientationVertical;
    self.recentStack.alignment = NSLayoutAttributeLeading;
    self.recentStack.distribution = NSStackViewDistributionFill;
    self.recentStack.spacing = 2.0;
    self.recentStack.frame = NSMakeRect(0.0, 0.0, 530.0, 0.0);
    self.recentScroll.documentView = self.recentStack;
    [content addArrangedSubview:self.recentScroll];
    [NSLayoutConstraint activateConstraints:@[
        [self.recentScroll.heightAnchor constraintEqualToConstant:300.0],
        [self.recentScroll.widthAnchor constraintEqualToConstant:540.0]
    ]];
    self.emptyRecentLabel = welcome_label(@"No recent documents yet.", 12.0, NSFontWeightRegular);
    self.emptyRecentLabel.textColor = NSColor.secondaryLabelColor;
    [self refresh];
    return self;
}

- (void)showForOwner:(platform_window *)owner
{
    self.owner = owner;
    self.dropView.owner = owner;
    [self refresh];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

- (void)refresh
{
    if (self.recentStack == nil) return;
    [self.recentStack.arrangedSubviews makeObjectsPerformSelector:@selector(removeFromSuperview)];
    [self.recentStack setFrameSize:NSMakeSize(530.0, 0.0)];
    size_t count = platform_recents_count();
    if (count == 0U) {
        [self.recentStack addArrangedSubview:self.emptyRecentLabel];
        [self.recentStack setFrameSize:NSMakeSize(530.0, 36.0)];
        return;
    }
    size_t height = 0U;
    for (size_t index = 0U; index < count && index < WELCOME_RECENT_LIMIT; ++index) {
        platform_recent_item item = {0};
        if (!platform_recents_get(index, &item)) continue;
        NSString *path = [[NSString alloc] initWithBytes:item.path length:item.path_length
            encoding:NSUTF8StringEncoding];
        if (path == nil || item.name_offset > item.path_length ||
            item.name_length > item.path_length - item.name_offset) continue;
        NSString *name = path.lastPathComponent;
        NSString *parent = item.parent_length == 0U ? @"" :
            path.stringByDeletingLastPathComponent;
        NSStackView *row = welcome_stack(NSUserInterfaceLayoutOrientationHorizontal, 8.0);
        NSStackView *labels = welcome_stack(NSUserInterfaceLayoutOrientationVertical, 2.0);
        NSTextField *nameField = welcome_label(name, 13.0, NSFontWeightMedium);
        nameField.lineBreakMode = NSLineBreakByTruncatingMiddle;
        nameField.toolTip = path;
        NSTextField *parentField = welcome_label(item.available ? parent :
            [NSString stringWithFormat:@"Unavailable · %@", parent], 11.0, NSFontWeightRegular);
        parentField.textColor = item.available ? NSColor.secondaryLabelColor : NSColor.systemRedColor;
        parentField.lineBreakMode = NSLineBreakByTruncatingMiddle;
        parentField.toolTip = path;
        [labels addArrangedSubview:nameField];
        [labels addArrangedSubview:parentField];
        VexelWelcomeButton *open = welcome_path_button(@"Open", self,
            @selector(openRecent:));
        VexelWelcomeButton *openButton = open;
        openButton.path = path;
        [row addArrangedSubview:labels];
        [row addArrangedSubview:open];
        if (!item.available) {
            VexelWelcomeButton *remove = welcome_path_button(@"Remove", self,
                @selector(removeRecent:));
            remove.bezelStyle = NSBezelStyleInline;
            VexelWelcomeButton *removeButton = remove;
            removeButton.path = path;
            [row addArrangedSubview:remove];
        }
        [self.recentStack addArrangedSubview:row];
        height += 48U;
    }
    [self.recentStack setFrameSize:NSMakeSize(530.0, (CGFloat)height)];
}

- (void)openPanel:(id)sender
{
    (void)sender;
    (void)platform_window_open_panel(self.owner);
}

- (void)newDocument:(id)sender
{
    (void)sender;
    platform_window_dispatch_action(self.owner, PLATFORM_ACTION_NEW_DOCUMENT);
}

- (void)openRecent:(id)sender
{
    if (![sender isKindOfClass:[VexelWelcomeButton class]]) return;
    NSString *path = [(VexelWelcomeButton *)sender path];
    const char *bytes = path.fileSystemRepresentation;
    size_t length = welcome_bounded_length(bytes, WELCOME_RECENT_PATH_CAP - 1U);
    if (platform_window_dispatch_file_open(self.owner, bytes, length)) {
        platform_macos_welcome_hide();
    }
}

- (void)removeRecent:(id)sender
{
    if (![sender isKindOfClass:[VexelWelcomeButton class]]) return;
    NSString *path = [(VexelWelcomeButton *)sender path];
    const char *bytes = path.fileSystemRepresentation;
    size_t length = welcome_bounded_length(bytes, WELCOME_RECENT_PATH_CAP - 1U);
    (void)platform_recents_remove(bytes, length);
}

- (void)clearRecent:(id)sender
{
    (void)sender;
    platform_recents_clear();
}

- (BOOL)windowShouldClose:(id)sender
{
    (void)sender;
    platform_window_dispatch_action(self.owner, PLATFORM_ACTION_CLOSE_DOCUMENT);
    return NO;
}

- (void)windowWillClose:(NSNotification *)notification
{
    (void)notification;
    self.owner = NULL;
    self.dropView.owner = NULL;
}
@end

void platform_macos_welcome_show(platform_window *owner)
{
    if (![NSThread isMainThread]) return;
    if (welcome_controller == nil)
        welcome_controller = [[VexelWelcomeController alloc] init];
    [welcome_controller showForOwner:owner];
}

void platform_macos_welcome_hide(void)
{
    if (![NSThread isMainThread] || welcome_controller.window == nil) return;
    [welcome_controller.window orderOut:nil];
}

void platform_macos_welcome_set_owner(platform_window *owner)
{
    if (![NSThread isMainThread]) return;
    welcome_controller.owner = owner;
    welcome_controller.dropView.owner = owner;
}

void platform_macos_welcome_clear_owner(platform_window *owner)
{
    if (![NSThread isMainThread] || welcome_controller.owner != owner) return;
    [welcome_controller.window orderOut:nil];
    welcome_controller.owner = NULL;
    welcome_controller.dropView.owner = NULL;
}

void platform_macos_welcome_refresh(void)
{
    if (![NSThread isMainThread] || welcome_controller == nil) return;
    [welcome_controller refresh];
}

void platform_macos_welcome_open_failure(platform_window *owner,
                                         const char *path, size_t path_length)
{
    if (![NSThread isMainThread] || owner == NULL ||
        !platform_recents_contains(path, path_length)) return;
    NSString *full_path = [[NSString alloc] initWithBytes:path length:path_length
        encoding:NSUTF8StringEncoding];
    if (full_path == nil) return;
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"Couldn't Open SVG";
    alert.informativeText = [NSString stringWithFormat:@"%@ could not be opened. Remove it from Recent Documents?",
        full_path.lastPathComponent];
    [alert addButtonWithTitle:@"Remove from Recent Documents"];
    [alert addButtonWithTitle:@"Keep"];
    NSWindow *parent = NSApp.keyWindow;
    if (parent == nil) parent = NSApp.mainWindow;
    if (parent == nil) return;
    [alert beginSheetModalForWindow:parent completionHandler:^(NSModalResponse response) {
        if (response == NSAlertFirstButtonReturn) {
            const char *stored_path = full_path.fileSystemRepresentation;
            size_t stored_length = welcome_bounded_length(stored_path,
                WELCOME_RECENT_PATH_CAP - 1U);
            (void)platform_recents_remove(stored_path, stored_length);
        }
    }];
}

#if defined(VEXEL_NATIVE_UI_GOLDEN_TEST)
NSView *platform_welcome_golden_view(void)
{
    static VexelWelcomeController *controller;
    if (controller == nil) controller = [[VexelWelcomeController alloc] init];
    return controller.window.contentView;
}
#endif
