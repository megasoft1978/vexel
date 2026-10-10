#import <Cocoa/Cocoa.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform_text_ui.h"
#include "platform/media/platform_text.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define UI_TEXT_CACHE_ENTRIES 64U
#define UI_TEXT_CACHE_BYTES (4U * 1024U * 1024U)
#define UI_TEXT_MAX_FONT_PX 256U
#define UI_TEXT_MAX_RENDER_HEIGHT 1024U

@class VexelAppearanceWatcher;

typedef struct ui_text_cache_entry {
    uint8_t *text;
    uint8_t *coverage;
    size_t text_length;
    size_t coverage_length;
    uint32_t width;
    uint32_t height;
    uint32_t clip_width;
    uint32_t font_size_px;
    ui_theme theme;
    ui_font_family family;
    ui_font_weight weight;
    uint64_t last_use;
} ui_text_cache_entry;

static ui_text_cache_entry ui_text_cache[UI_TEXT_CACHE_ENTRIES];
static size_t ui_text_cache_bytes;
static uint64_t ui_text_cache_clock;
static platform_text_ui_preferences_callback preferences_callback;
static void *preferences_context;
static VexelAppearanceWatcher *watcher_view;
static id accessibility_options_observer;
static bool appearance_kvo_installed;
static char appearance_kvo_context;

static bool ui_text_bytes_valid(const char *bytes, size_t length)
{
    return length <= PLATFORM_TEXT_MAX_BYTES && (length == 0U || bytes != NULL);
}

static size_t utf8_codepoint_count(const char *text, size_t length)
{
    size_t count = 0U;
    for (size_t index = 0U; index < length; ++index)
        if (((uint8_t)text[index] & 0xc0U) != 0x80U) ++count;
    return count;
}

static size_t utf8_bytes_for_codepoints(const char *text, size_t length,
                                        size_t wanted)
{
    size_t count = 0U;
    for (size_t index = 0U; index < length; ++index) {
        if (((uint8_t)text[index] & 0xc0U) == 0x80U) continue;
        if (count == wanted) return index;
        ++count;
    }
    return length;
}

static bool text_measure(const char *text, size_t length, ui_font_family family,
                         ui_font_weight weight, uint32_t font_size_px,
                         double *out_width)
{
    static const uint8_t sf_text[] = "SF Pro Text";
    static const uint8_t sf_mono[] = "SF Mono";
    if (out_width == NULL) return false;
    platform_text_slice font_family = family == UI_FONT_MONOSPACED ?
        (platform_text_slice){sf_mono, sizeof(sf_mono) - 1U} :
        (platform_text_slice){sf_text, sizeof(sf_text) - 1U};
    platform_text_request request = {.text = {(const uint8_t *)text, length},
        .font_families = &font_family, .font_family_count = 1U,
        .size = (double)font_size_px,
        .weight = weight == UI_FONT_SEMIBOLD ? 600.0 : 400.0,
        .style = PLATFORM_TEXT_STYLE_NORMAL, .letter_spacing = 0.0};
    platform_text_result layout = PLATFORM_TEXT_RESULT_INIT;
    if (platform_text_layout(&request, &layout) != PLATFORM_MEDIA_OK) return false;
    bool okay = isfinite(layout.advance_width) && layout.advance_width >= 0.0;
    if (okay) *out_width = layout.advance_width;
    platform_text_result_destroy(&layout);
    return okay;
}

static bool fitted_text(const char *text, size_t length, ui_font_family family,
    ui_font_weight weight, uint32_t font_size_px, uint32_t clip_width,
    const char **out_text, size_t *out_length, char **out_owned)
{
    static const char ellipsis[] = "\xe2\x80\xa6";
    *out_text = text;
    *out_length = length;
    *out_owned = NULL;
    size_t codepoints = utf8_codepoint_count(text, length);
    double width = 0.0;
    if (codepoints <= PLATFORM_TEXT_MAX_GLYPHS &&
        text_measure(text, length, family, weight, font_size_px, &width) &&
        width <= (double)clip_width) return true;
    double ellipsis_width = 0.0;
    if (!text_measure(ellipsis, sizeof(ellipsis) - 1U, family, weight,
            font_size_px, &ellipsis_width)) return false;
    if (ellipsis_width > (double)clip_width) {
        *out_text = "";
        *out_length = 0U;
        return true;
    }
    size_t maximum = codepoints < PLATFORM_TEXT_MAX_GLYPHS ? codepoints :
        PLATFORM_TEXT_MAX_GLYPHS - 1U;
    size_t low = 0U, high = maximum;
    uint32_t available = clip_width - (uint32_t)ceil(ellipsis_width);
    while (low < high) {
        size_t middle = low + (high - low + 1U) / 2U;
        size_t prefix_length = utf8_bytes_for_codepoints(text, length, middle);
        double prefix_width = 0.0;
        if (text_measure(text, prefix_length, family, weight, font_size_px,
                &prefix_width) && prefix_width <= (double)available) low = middle;
        else high = middle - 1U;
    }
    size_t prefix_length = utf8_bytes_for_codepoints(text, length, low);
    if (prefix_length > SIZE_MAX - (sizeof(ellipsis) - 1U)) return false;
    size_t fitted_length = prefix_length + sizeof(ellipsis) - 1U;
    char *fitted = malloc(fitted_length);
    if (fitted == NULL) return false;
    if (prefix_length > 0U) memcpy(fitted, text, prefix_length);
    memcpy(fitted + prefix_length, ellipsis, sizeof(ellipsis) - 1U);
    *out_text = fitted;
    *out_length = fitted_length;
    *out_owned = fitted;
    return true;
}

static ui_theme current_theme(ui_theme fallback)
{
    if (NSApp == nil) return fallback;
    NSAppearance *appearance = NSApp.effectiveAppearance;
    NSAppearanceName matched = [appearance bestMatchFromAppearancesWithNames:@[
        NSAppearanceNameDarkAqua, NSAppearanceNameAqua]];
    return [matched isEqualToString:NSAppearanceNameDarkAqua] ? UI_THEME_DARK : UI_THEME_LIGHT;
}

bool platform_text_ui_app_is_active(void)
{
    return [NSThread isMainThread] && NSApp != nil;
}

bool platform_text_ui_get_preferences(ui_theme *out_appearance,
                                      bool *out_increased_contrast,
                                      bool *out_reduced_motion)
{
    if (out_appearance == NULL || out_increased_contrast == NULL ||
        out_reduced_motion == NULL || ![NSThread isMainThread] || NSApp == nil)
        return false;
    *out_appearance = current_theme(UI_THEME_DARK);
    NSWorkspace *workspace = [NSWorkspace sharedWorkspace];
    *out_increased_contrast = workspace.accessibilityDisplayShouldIncreaseContrast;
    *out_reduced_motion = workspace.accessibilityDisplayShouldReduceMotion;
    return true;
}

@interface VexelAppearanceWatcher : NSView
@property(nonatomic, assign) platform_text_ui_preferences_callback callback;
@property(nonatomic, assign) void *callbackContext;
- (void)notifyPreferences;
@end

@implementation VexelAppearanceWatcher
- (void)viewDidChangeEffectiveAppearance
{
    [super viewDidChangeEffectiveAppearance];
    [self notifyPreferences];
}
- (NSView *)hitTest:(NSPoint)point
{
    (void)point;
    return nil;
}
- (void)notifyPreferences
{
    if (self.callback == NULL) return;
    ui_theme theme = UI_THEME_DARK;
    bool increased = false, reduced = false;
    if (platform_text_ui_get_preferences(&theme, &increased, &reduced))
        self.callback(theme, increased, reduced, self.callbackContext);
}
- (void)observeValueForKeyPath:(NSString *)keyPath ofObject:(id)object
                        change:(NSDictionary<NSKeyValueChangeKey, id> *)change
                       context:(void *)context
{
    (void)object;
    (void)change;
    if (context == &appearance_kvo_context &&
        [keyPath isEqualToString:@"effectiveAppearance"]) {
        [self notifyPreferences];
        return;
    }
    [super observeValueForKeyPath:keyPath ofObject:object change:change context:context];
}
@end

static VexelAppearanceWatcher *appearance_watcher(void)
{
    if (preferences_callback == NULL || NSApp == nil || ![NSThread isMainThread])
        return nil;
    NSWindow *window = NSApp.keyWindow;
    NSView *content = window.contentView;
    if (content == nil) return nil;
    if (watcher_view == nil) {
        /* This is a 1×1 transparent observer view; it inherits AppKit appearance. */
        watcher_view = [[VexelAppearanceWatcher alloc]
            initWithFrame:NSMakeRect(0.0, 0.0, 1.0, 1.0)];
        watcher_view.alphaValue = 0.0;
        watcher_view.callback = preferences_callback;
        watcher_view.callbackContext = preferences_context;
        [content addSubview:watcher_view positioned:NSWindowBelow relativeTo:nil];
    } else if (watcher_view.superview != content) {
        [watcher_view removeFromSuperview];
        [content addSubview:watcher_view positioned:NSWindowBelow relativeTo:nil];
    }
    watcher_view.callback = preferences_callback;
    watcher_view.callbackContext = preferences_context;
    return watcher_view;
}

bool platform_text_ui_watch_preferences(platform_text_ui_preferences_callback callback,
                                        void *context)
{
    if (callback == NULL || context == NULL || ![NSThread isMainThread] || NSApp == nil)
        return false;
    preferences_callback = callback;
    preferences_context = context;
    VexelAppearanceWatcher *watcher = appearance_watcher();
    if (watcher == nil) {
        preferences_callback = NULL;
        preferences_context = NULL;
        return false;
    }
    if (!appearance_kvo_installed) {
        @try {
            [NSApp addObserver:watcher forKeyPath:@"effectiveAppearance"
                options:NSKeyValueObservingOptionNew context:&appearance_kvo_context];
            appearance_kvo_installed = true;
        } @catch (NSException *exception) {
            (void)exception;
        }
    }
    if (accessibility_options_observer == nil) {
        accessibility_options_observer = [NSWorkspace.sharedWorkspace.notificationCenter
            addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification
            object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *note) {
                (void)note;
                [watcher_view notifyPreferences];
            }];
    }
    [watcher notifyPreferences];
    return true;
}

void platform_text_ui_unwatch_preferences(void *context)
{
    if (![NSThread isMainThread] || context == NULL || preferences_context != context)
        return;
    watcher_view.callback = NULL;
    watcher_view.callbackContext = NULL;
    if (appearance_kvo_installed) {
        @try {
            [NSApp removeObserver:watcher_view forKeyPath:@"effectiveAppearance"
                context:&appearance_kvo_context];
        } @catch (NSException *exception) {
            (void)exception;
        }
        appearance_kvo_installed = false;
    }
    if (accessibility_options_observer != nil) {
        [NSWorkspace.sharedWorkspace.notificationCenter
            removeObserver:accessibility_options_observer];
        accessibility_options_observer = nil;
    }
    [watcher_view removeFromSuperview];
    watcher_view = nil;
    preferences_callback = NULL;
    preferences_context = NULL;
}

static CGPathRef path_create(const platform_text_result *layout)
{
    CGMutablePathRef path = CGPathCreateMutable();
    if (path == NULL) return NULL;
    for (size_t index = 0U; index < layout->command_count; ++index) {
        const platform_text_path_command *command = &layout->commands[index];
        switch (command->verb) {
            case PLATFORM_TEXT_PATH_MOVE:
                CGPathMoveToPoint(path, NULL, command->values[0], command->values[1]);
                break;
            case PLATFORM_TEXT_PATH_LINE:
                CGPathAddLineToPoint(path, NULL, command->values[0], command->values[1]);
                break;
            case PLATFORM_TEXT_PATH_CUBIC:
                CGPathAddCurveToPoint(path, NULL, command->values[0], command->values[1],
                    command->values[2], command->values[3], command->values[4],
                    command->values[5]);
                break;
            case PLATFORM_TEXT_PATH_CLOSE:
                CGPathCloseSubpath(path);
                break;
        }
    }
    return path;
}

static void cache_remove(ui_text_cache_entry *entry)
{
    if (entry->coverage != NULL) {
        ui_text_cache_bytes -= entry->text_length + entry->coverage_length;
    }
    free(entry->text);
    free(entry->coverage);
    *entry = (ui_text_cache_entry){0};
}

static ui_text_cache_entry *cache_find(const char *text, size_t length,
    ui_font_family family, ui_font_weight weight, ui_theme theme,
    uint32_t font_size_px, uint32_t max_width)
{
    for (size_t index = 0U; index < UI_TEXT_CACHE_ENTRIES; ++index) {
        ui_text_cache_entry *entry = &ui_text_cache[index];
        if (entry->coverage == NULL || entry->text_length != length ||
            entry->family != family || entry->weight != weight || entry->theme != theme ||
            entry->font_size_px != font_size_px || entry->clip_width != max_width ||
            (length > 0U && memcmp(entry->text, text, length) != 0)) continue;
        entry->last_use = ++ui_text_cache_clock;
        return entry;
    }
    return NULL;
}

static void cache_insert(const char *text, size_t length, ui_font_family family,
    ui_font_weight weight, ui_theme theme, uint32_t font_size_px,
    uint32_t width, uint32_t height,
    uint32_t clip_width,
    uint8_t *coverage)
{
    size_t coverage_length = (size_t)width * height;
    if (length > UI_TEXT_CACHE_BYTES || coverage_length > UI_TEXT_CACHE_BYTES - length)
        return;
    size_t slot = 0U;
    for (size_t index = 0U; index < UI_TEXT_CACHE_ENTRIES; ++index) {
        if (ui_text_cache[index].coverage == NULL) { slot = index; goto selected; }
        if (ui_text_cache[index].last_use < ui_text_cache[slot].last_use) slot = index;
    }
selected:
    cache_remove(&ui_text_cache[slot]);
    while (ui_text_cache_bytes + length + coverage_length > UI_TEXT_CACHE_BYTES) {
        size_t oldest = UI_TEXT_CACHE_ENTRIES;
        for (size_t index = 0U; index < UI_TEXT_CACHE_ENTRIES; ++index) {
            if (index == slot || ui_text_cache[index].coverage == NULL) continue;
            if (oldest == UI_TEXT_CACHE_ENTRIES ||
                ui_text_cache[index].last_use < ui_text_cache[oldest].last_use) oldest = index;
        }
        if (oldest == UI_TEXT_CACHE_ENTRIES) break;
        cache_remove(&ui_text_cache[oldest]);
    }
    uint8_t *text_copy = length == 0U ? NULL : malloc(length);
    uint8_t *coverage_copy = malloc(coverage_length);
    if ((length > 0U && text_copy == NULL) || coverage_copy == NULL) {
        free(text_copy);
        free(coverage_copy);
        return;
    }
    if (length > 0U) memcpy(text_copy, text, length);
    memcpy(coverage_copy, coverage, coverage_length);
    ui_text_cache[slot] = (ui_text_cache_entry){.text = text_copy,
        .coverage = coverage_copy, .text_length = length,
        .coverage_length = coverage_length, .width = width, .height = height,
        .clip_width = clip_width,
        .font_size_px = font_size_px, .theme = theme,
        .family = family, .weight = weight, .last_use = ++ui_text_cache_clock};
    ui_text_cache_bytes += length + coverage_length;
}

static bool make_coverage(const char *text, size_t length, ui_font_family family,
    ui_font_weight weight, uint32_t font_size_px, uint32_t max_width,
    uint8_t **out_coverage,
    uint32_t *out_width, uint32_t *out_height)
{
    static const uint8_t sf_text[] = "SF Pro Text";
    static const uint8_t sf_mono[] = "SF Mono";
    platform_text_slice font_family = family == UI_FONT_MONOSPACED ?
        (platform_text_slice){sf_mono, sizeof(sf_mono) - 1U} :
        (platform_text_slice){sf_text, sizeof(sf_text) - 1U};
    platform_text_request request = {.text = {
            (const uint8_t *)text, length}, .font_families = &font_family,
        .font_family_count = 1U, .size = (double)font_size_px,
        .weight = weight == UI_FONT_SEMIBOLD ? 600.0 : 400.0,
        .style = PLATFORM_TEXT_STYLE_NORMAL,
        .letter_spacing = 0.0};
    platform_text_result layout = PLATFORM_TEXT_RESULT_INIT;
    if (platform_text_layout(&request, &layout) != PLATFORM_MEDIA_OK) return false;
    double line_height = ceil(layout.ascent + layout.descent + layout.leading);
    double measured_width = ceil(layout.advance_width);
    if (!isfinite(line_height) || !isfinite(measured_width) ||
        line_height < 1.0 || line_height > UI_TEXT_MAX_RENDER_HEIGHT ||
        measured_width < 0.0 || measured_width > 65536.0) {
        platform_text_result_destroy(&layout);
        return false;
    }
    uint32_t width = (uint32_t)measured_width;
    if (width == 0U && layout.glyph_count > 0U) width = 1U;
    if (width > max_width) width = max_width;
    uint32_t height = (uint32_t)line_height;
    if (width == 0U || height == 0U || (size_t)width > SIZE_MAX / height) {
        platform_text_result_destroy(&layout);
        return false;
    }
    size_t byte_count = (size_t)width * height;
    uint8_t *coverage = calloc(byte_count, 1U);
    CGColorSpaceRef color_space = CGColorSpaceCreateDeviceGray();
    CGContextRef context = coverage == NULL || color_space == NULL ? NULL :
        CGBitmapContextCreate(coverage, width, height, 8U, width, color_space,
            (CGBitmapInfo)kCGImageAlphaNone);
    if (context == NULL) {
        free(coverage);
        if (color_space != NULL) CGColorSpaceRelease(color_space);
        platform_text_result_destroy(&layout);
        return false;
    }
    CGContextSetShouldAntialias(context, true);
    CGContextSetGrayFillColor(context, 1.0, 1.0);
    CGContextTranslateCTM(context, 0.0, (double)height);
    CGContextScaleCTM(context, 1.0, -1.0);
    CGPathRef path = path_create(&layout);
    if (path == NULL) {
        CGContextRelease(context);
        CGColorSpaceRelease(color_space);
        free(coverage);
        platform_text_result_destroy(&layout);
        return false;
    }
    CGAffineTransform top_down = CGAffineTransformMake(1.0, 0.0, 0.0, -1.0,
        0.0, ceil(layout.ascent));
    CGPathRef screen_path = CGPathCreateMutableCopyByTransformingPath(path, &top_down);
    if (screen_path == NULL) {
        CGPathRelease(path);
        CGContextRelease(context);
        CGColorSpaceRelease(color_space);
        free(coverage);
        platform_text_result_destroy(&layout);
        return false;
    }
    CGContextAddPath(context, screen_path);
    CGContextFillPath(context);
    CGPathRelease(screen_path);
    CGPathRelease(path);
    CGContextRelease(context);
    CGColorSpaceRelease(color_space);
    platform_text_result_destroy(&layout);
    *out_coverage = coverage;
    *out_width = width;
    *out_height = height;
    return true;
}

static void blend_coverage(uint8_t *destination, uint32_t color, uint8_t coverage)
{
    uint32_t alpha = ((color & 255U) * coverage + 127U) / 255U;
    uint32_t inverse = 255U - alpha;
    uint32_t destination_alpha = destination[3];
    uint32_t out_alpha = alpha + (destination_alpha * inverse + 127U) / 255U;
    if (out_alpha == 0U) return;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        uint32_t numerator = source * alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha * inverse;
        destination[channel] = (uint8_t)((numerator + out_alpha * 127U) /
            (out_alpha * 255U));
    }
    destination[3] = (uint8_t)out_alpha;
}

bool platform_text_ui_draw(const char *utf8, size_t length,
                           ui_font_family family, ui_font_weight weight,
                           bool centered, ui_theme theme,
                           uint32_t font_size_px, uint32_t color,
                           uint8_t *rgba, uint32_t width, uint32_t height,
                           size_t stride, ui_rect clip)
{
    if (!ui_text_bytes_valid(utf8, length) || theme > UI_THEME_LIGHT ||
        (family != UI_FONT_SYSTEM && family != UI_FONT_MONOSPACED) ||
        (weight != UI_FONT_REGULAR && weight != UI_FONT_SEMIBOLD) ||
        font_size_px == 0U || font_size_px > UI_TEXT_MAX_FONT_PX || rgba == NULL ||
        width == 0U || height == 0U || (size_t)width > SIZE_MAX / 4U ||
        stride < (size_t)width * 4U || (size_t)height > SIZE_MAX / stride ||
        clip.x > width || clip.y > height || clip.width > width - clip.x ||
        clip.height > height - clip.y) return false;
    if (length == 0U || clip.width == 0U || clip.height == 0U) return true;
    uint32_t max_width = clip.width;
    const char *draw_text = NULL;
    size_t draw_length = 0U;
    char *owned_text = NULL;
    if (!fitted_text(utf8, length, family, weight, font_size_px, max_width,
            &draw_text, &draw_length, &owned_text)) return false;
    if (draw_length == 0U) { free(owned_text); return true; }
    ui_text_cache_entry *entry = cache_find(draw_text, draw_length, family,
        weight, theme, font_size_px, max_width);
    uint8_t *fresh_coverage = NULL;
    uint32_t text_width = 0U, text_height = 0U;
    const uint8_t *coverage = NULL;
    if (entry != NULL) {
        coverage = entry->coverage;
        text_width = entry->width;
        text_height = entry->height;
    } else {
        if (!make_coverage(draw_text, draw_length, family, weight, font_size_px,
                max_width, &fresh_coverage, &text_width, &text_height)) {
            free(owned_text);
            return false;
        }
        coverage = fresh_coverage;
        cache_insert(draw_text, draw_length, family, weight, theme, font_size_px,
            text_width, text_height, max_width, fresh_coverage);
    }
    uint32_t draw_height = text_height < clip.height ? text_height : clip.height;
    uint32_t origin_x = clip.x;
    if (centered && text_width < clip.width) origin_x += (clip.width - text_width) / 2U;
    uint32_t origin_y = clip.y + (clip.height > draw_height ?
        (clip.height - draw_height) / 2U : 0U);
    for (uint32_t y = 0U; y < draw_height; ++y) {
        for (uint32_t x = 0U; x < text_width; ++x) {
            uint32_t px = origin_x + x;
            if (px >= clip.x + clip.width || px >= width) break;
            uint8_t alpha = coverage[(size_t)y * text_width + x];
            if (alpha != 0U)
                blend_coverage(rgba + (size_t)(origin_y + y) * stride +
                    (size_t)px * 4U, color, alpha);
        }
    }
    free(fresh_coverage);
    free(owned_text);
    return true;
}

bool platform_text_ui_draw_widget(void *context, const char *utf8, size_t length,
    ui_rect bounds, ui_theme theme, uint32_t font_size_px, uint32_t color,
    ui_font_family family, ui_font_weight weight, uint8_t *rgba,
    uint32_t width, uint32_t height, size_t stride)
{
    (void)context;
    return platform_text_ui_draw(utf8, length, family, weight, false, theme,
        font_size_px, color, rgba, width, height, stride, bounds);
}

void platform_text_ui_cache_clear(void)
{
    for (size_t index = 0U; index < UI_TEXT_CACHE_ENTRIES; ++index)
        cache_remove(&ui_text_cache[index]);
    ui_text_cache_bytes = 0U;
    ui_text_cache_clock = 0U;
}
