#import <Cocoa/Cocoa.h>

#include "app/feedback_diagnostics.h"
#include "platform/platform_feedback.h"
#include "platform/platform_settings.h"

#include <string.h>

#ifndef VEXEL_VERSION_STRING
#define VEXEL_VERSION_STRING "0.0.1"
#endif

@interface VexelHelpController : NSObject <NSWindowDelegate>
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, assign) platform_window *owner;
- (void)showForOwner:(platform_window *)owner;
- (void)openDocument:(id)sender;
- (void)sendFeedback:(id)sender;
@end

@interface VexelFeedbackController : NSObject <NSWindowDelegate>
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) NSTextView *preview;
@property(nonatomic, strong) NSButton *includeOptionalDetails;
- (void)show;
- (void)refreshPreview;
- (void)copyDiagnostics:(id)sender;
- (void)createEmail:(id)sender;
- (void)toggleOptionalDetails:(id)sender;
@end

static VexelHelpController *help_controller;
static VexelFeedbackController *feedback_controller;

static NSString *feedback_app_version(void)
{
    NSString *version = NSBundle.mainBundle.infoDictionary[@"CFBundleShortVersionString"];
    return version.length > 0U ? version : @VEXEL_VERSION_STRING;
}

static NSString *feedback_report(BOOL include_optional)
{
    NSString *version = feedback_app_version();
    NSString *system = NSProcessInfo.processInfo.operatingSystemVersionString;
    NSData *version_bytes = [version dataUsingEncoding:NSUTF8StringEncoding];
    NSData *system_bytes = [system dataUsingEncoding:NSUTF8StringEncoding];
    if (version_bytes == nil || system_bytes == nil) return nil;
    platform_settings_values preferences = {0};
    platform_settings_get(&preferences);
    char text[512];
    size_t length = 0U;
    if (!feedback_diagnostics_build(version_bytes.bytes, version_bytes.length,
            system_bytes.bytes, system_bytes.length, include_optional,
            preferences.default_checkerboard, preferences.default_pixel_grid,
            text, sizeof(text), &length)) return nil;
    return [[NSString alloc] initWithBytes:text length:length encoding:NSUTF8StringEncoding];
}

static NSTextField *help_label(NSString *text, CGFloat size, NSFontWeight weight)
{
    NSTextField *label = [NSTextField labelWithString:text];
    label.font = [NSFont systemFontOfSize:size weight:weight];
    label.translatesAutoresizingMaskIntoConstraints = NO;
    return label;
}

@implementation VexelHelpController
- (instancetype)init
{
    self = [super init];
    if (self == nil) return nil;
    NSRect frame = NSMakeRect(0.0, 0.0, 520.0, 360.0);
    self.window = [[NSWindow alloc] initWithContentRect:frame
        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
        backing:NSBackingStoreBuffered defer:NO];
    self.window.releasedWhenClosed = NO;
    self.window.title = @"Vexel Help";
    self.window.delegate = self;
    NSView *root = [[NSView alloc] initWithFrame:frame];
    root.translatesAutoresizingMaskIntoConstraints = NO;
    self.window.contentView = root;
    NSTextField *heading = help_label(@"Working with SVGs in Vexel", 21.0, NSFontWeightBold);
    NSTextField *body = help_label(@"Open or drag in an SVG to inspect its structure and edit supported elements. Use the canvas to select, move, and resize artwork. Save preserves the SVG source outside the edited spans.", 13.0, NSFontWeightRegular);
    body.maximumNumberOfLines = 0;
    body.lineBreakMode = NSLineBreakByWordWrapping;
    [body.widthAnchor constraintEqualToConstant:460.0].active = YES;
    NSTextField *shortcut_heading = help_label(@"Keyboard shortcuts", 15.0,
        NSFontWeightSemibold);
    NSTextField *shortcuts = help_label(@"Open  ⌘O     Save  ⌘S     Settings  ⌘,\nFit in Window  ⌘0     Actual Size  ⌘1\nFull Screen  Control-⌘-F",
        12.0, NSFontWeightRegular);
    shortcuts.maximumNumberOfLines = 0;
    NSButton *open = [NSButton buttonWithTitle:@"Open SVG…" target:self action:@selector(openDocument:)];
    NSButton *feedback = [NSButton buttonWithTitle:@"Send Feedback…" target:self action:@selector(sendFeedback:)];
    NSStackView *stack = [[NSStackView alloc] initWithFrame:NSZeroRect];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.spacing = 16.0;
    stack.edgeInsets = NSEdgeInsetsMake(28.0, 30.0, 28.0, 30.0);
    stack.translatesAutoresizingMaskIntoConstraints = NO;
    [stack addArrangedSubview:heading];
    [stack addArrangedSubview:body];
    [stack addArrangedSubview:shortcut_heading];
    [stack addArrangedSubview:shortcuts];
    NSStackView *buttons = [[NSStackView alloc] initWithFrame:NSZeroRect];
    buttons.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    buttons.spacing = 10.0;
    [buttons addArrangedSubview:open];
    [buttons addArrangedSubview:feedback];
    [stack addArrangedSubview:buttons];
    [root addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:root.topAnchor],
        [stack.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [stack.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],
        [stack.bottomAnchor constraintLessThanOrEqualToAnchor:root.bottomAnchor]
    ]];
    return self;
}

- (void)showForOwner:(platform_window *)owner
{
    self.owner = owner;
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

- (void)openDocument:(id)sender
{
    (void)sender;
    (void)platform_window_open_panel(self.owner);
}

- (void)sendFeedback:(id)sender
{
    (void)sender;
    platform_feedback_show();
}

- (void)windowWillClose:(NSNotification *)notification
{
    (void)notification;
    self.owner = NULL;
}
@end

@implementation VexelFeedbackController
- (instancetype)init
{
    self = [super init];
    if (self == nil) return nil;
    NSRect frame = NSMakeRect(0.0, 0.0, 620.0, 480.0);
    self.window = [[NSWindow alloc] initWithContentRect:frame
        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
        backing:NSBackingStoreBuffered defer:NO];
    self.window.releasedWhenClosed = NO;
    self.window.title = @"Send Feedback";
    self.window.delegate = self;
    NSView *root = [[NSView alloc] initWithFrame:frame];
    root.translatesAutoresizingMaskIntoConstraints = NO;
    self.window.contentView = root;
    NSTextField *intro = help_label(@"Review the message before creating an email. Vexel never adds your document name, path, or SVG source.", 13.0, NSFontWeightRegular);
    intro.maximumNumberOfLines = 0;
    intro.lineBreakMode = NSLineBreakByWordWrapping;
    [intro.widthAnchor constraintEqualToConstant:576.0].active = YES;
    self.includeOptionalDetails = [NSButton checkboxWithTitle:@"Include optional diagnostics"
        target:self action:@selector(toggleOptionalDetails:)];
    self.includeOptionalDetails.state = NSControlStateValueOff;
    NSScrollView *scroll = [[NSScrollView alloc] initWithFrame:NSZeroRect];
    scroll.translatesAutoresizingMaskIntoConstraints = NO;
    scroll.hasVerticalScroller = YES;
    scroll.borderType = NSBezelBorder;
    self.preview = [[NSTextView alloc] initWithFrame:NSZeroRect];
    self.preview.editable = NO;
    self.preview.selectable = YES;
    self.preview.font = [NSFont monospacedSystemFontOfSize:12.0 weight:NSFontWeightRegular];
    scroll.documentView = self.preview;
    NSButton *copy = [NSButton buttonWithTitle:@"Copy Diagnostics"
        target:self action:@selector(copyDiagnostics:)];
    NSButton *email = [NSButton buttonWithTitle:@"Create Email…"
        target:self action:@selector(createEmail:)];
    email.keyEquivalent = @"\r";
    NSStackView *buttons = [[NSStackView alloc] initWithFrame:NSZeroRect];
    buttons.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    buttons.alignment = NSLayoutAttributeCenterY;
    buttons.spacing = 10.0;
    [buttons addArrangedSubview:copy];
    [buttons addArrangedSubview:email];
    NSStackView *stack = [[NSStackView alloc] initWithFrame:NSZeroRect];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.spacing = 12.0;
    stack.edgeInsets = NSEdgeInsetsMake(20.0, 22.0, 20.0, 22.0);
    stack.translatesAutoresizingMaskIntoConstraints = NO;
    [stack addArrangedSubview:intro];
    [stack addArrangedSubview:self.includeOptionalDetails];
    [stack addArrangedSubview:scroll];
    [stack addArrangedSubview:buttons];
    [root addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:root.topAnchor],
        [stack.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [stack.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],
        [stack.bottomAnchor constraintEqualToAnchor:root.bottomAnchor],
        [scroll.widthAnchor constraintEqualToConstant:576.0],
        [scroll.heightAnchor constraintEqualToConstant:300.0]
    ]];
    [self refreshPreview];
    return self;
}

- (void)show
{
    self.includeOptionalDetails.state = NSControlStateValueOff;
    [self refreshPreview];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

- (void)refreshPreview
{
    NSString *text = feedback_report(self.includeOptionalDetails.state == NSControlStateValueOn);
    self.preview.string = text == nil ? @"Diagnostics are unavailable." : text;
}

- (void)copyDiagnostics:(id)sender
{
    (void)sender;
    NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
    [pasteboard clearContents];
    [pasteboard setString:self.preview.string forType:NSPasteboardTypeString];
}

- (void)createEmail:(id)sender
{
    (void)sender;
    NSString *report = self.preview.string;
    if (report.length == 0U) return;
    NSMutableArray *items = [NSMutableArray arrayWithObject:report];
    if (self.includeOptionalDetails.state == NSControlStateValueOn) {
        NSString *filename = [NSString stringWithFormat:@"Vexel-Diagnostics-%@.txt",
            NSUUID.UUID.UUIDString];
        NSURL *attachment = [NSURL fileURLWithPath:[NSTemporaryDirectory()
            stringByAppendingPathComponent:filename]];
        NSError *error = nil;
        if (![report writeToURL:attachment atomically:YES encoding:NSUTF8StringEncoding
            error:&error]) {
            NSAlert *alert = [[NSAlert alloc] init];
            alert.messageText = @"Diagnostics Attachment Unavailable";
            alert.informativeText = @"Copy Diagnostics remains available. No message was sent.";
            [alert addButtonWithTitle:@"OK"];
            [alert runModal];
            return;
        }
        [items addObject:attachment];
    }
    NSSharingService *service = [NSSharingService
        sharingServiceNamed:NSSharingServiceNameComposeEmail];
    if (service == nil || ![service canPerformWithItems:items]) {
        NSAlert *alert = [[NSAlert alloc] init];
        alert.messageText = @"No Email Composer Available";
        alert.informativeText = @"Copy Diagnostics to share the reviewed report with your support contact.";
        [alert addButtonWithTitle:@"OK"];
        [alert runModal];
        return;
    }
    service.subject = @"Vexel Feedback";
    [service performWithItems:items];
}

- (void)toggleOptionalDetails:(id)sender
{
    (void)sender;
    [self refreshPreview];
}

- (void)windowWillClose:(NSNotification *)notification
{
    (void)notification;
}
@end

void platform_help_show(platform_window *owner_window)
{
    if (![NSThread isMainThread]) return;
    if (help_controller == nil) help_controller = [[VexelHelpController alloc] init];
    [help_controller showForOwner:owner_window];
}

void platform_feedback_show(void)
{
    if (![NSThread isMainThread]) return;
    if (feedback_controller == nil) feedback_controller = [[VexelFeedbackController alloc] init];
    [feedback_controller show];
}

void platform_feedback_copy_diagnostics(void)
{
    if (![NSThread isMainThread]) return;
    NSString *report = feedback_report(NO);
    if (report == nil) return;
    NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
    [pasteboard clearContents];
    [pasteboard setString:report forType:NSPasteboardTypeString];
}

#if defined(VEXEL_NATIVE_UI_GOLDEN_TEST)
NSView *platform_help_golden_view(void)
{
    static VexelHelpController *controller;
    if (controller == nil) controller = [[VexelHelpController alloc] init];
    return controller.window.contentView;
}

NSView *platform_feedback_golden_view(void)
{
    static VexelFeedbackController *controller;
    if (controller == nil) controller = [[VexelFeedbackController alloc] init];
    controller.preview.string = @"Vexel 0.0.1\nmacOS 13.0.0\nCanvas: checkerboard on, pixel grid off";
    return controller.window.contentView;
}
#endif
