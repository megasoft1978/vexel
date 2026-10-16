#import <Foundation/Foundation.h>

#include "platform_window_internal.h"

#include <stdio.h>
#include <string.h>

bool platform_window_emit_internal(platform_window *window, const platform_event *event)
{
    (void)window;
    (void)event;
    return false;
}

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

int main(void)
{
    static const char svg[] = "/tmp/drawing.svg";
    static const char project[] = "/tmp/cartoon.VEXEL";
    static const char unsupported[] = "/tmp/image.png";
    CHECK(platform_path_supported_internal(svg, sizeof(svg) - 1U));
    CHECK(platform_path_supported_internal(project, sizeof(project) - 1U));
    CHECK(!platform_path_supported_internal(unsupported, sizeof(unsupported) - 1U));
    CHECK(!platform_path_supported_internal(project, 0U));
    CHECK(!platform_path_supported_internal(project, PLATFORM_PATH_CAP));
    return 0;
}
