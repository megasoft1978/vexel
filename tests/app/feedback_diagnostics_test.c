#include "app/feedback_diagnostics.h"

#include <stdio.h>
#include <string.h>

static bool require(bool condition, const char *message)
{
    if (condition) return true;
    (void)fprintf(stderr, "feedback diagnostics: %s\n", message);
    return false;
}

int main(void)
{
    char report[512];
    size_t length = 0U;
    const char *version = "0.0.1";
    const char *os = "macOS 15.0";
    if (!feedback_diagnostics_build(version, strlen(version), os, strlen(os),
            false, true, false, report, sizeof(report), &length) ||
        !require(strstr(report, "Vexel version: 0.0.1") != NULL,
            "includes the application version") ||
        !require(strstr(report, "macOS version: macOS 15.0") != NULL,
            "includes the operating system version") ||
        !require(strstr(report, "Document names, paths, and SVG contents are excluded.") != NULL,
            "states that document data is excluded") ||
        !require(strstr(report, "Canvas defaults:") == NULL,
            "keeps optional diagnostics off by default") ||
        !require(length == strlen(report), "returns the exact preview length")) return 1;
    if (!feedback_diagnostics_build(version, strlen(version), os, strlen(os),
            true, true, false, report, sizeof(report), &length) ||
        !require(strstr(report, "Diagnostics format: 1") != NULL,
            "includes versioned optional diagnostics") ||
        !require(strstr(report, "Canvas defaults: checkerboard=on; pixel grid=off.") != NULL,
            "previews exact safe optional fields")) return 1;
    char too_small[8];
    if (!require(!feedback_diagnostics_build(version, strlen(version), os, strlen(os),
            false, false, false, too_small, sizeof(too_small), &length),
            "rejects a truncated preview")) return 1;
    const char embedded_nul[] = {'1', '\0', 'x'};
    if (!require(!feedback_diagnostics_build(embedded_nul, sizeof(embedded_nul),
            os, strlen(os), false, false, false, report, sizeof(report), &length),
            "rejects invalid version slices")) return 1;
    (void)puts("feedback diagnostics: bounded preview passed");
    return 0;
}
