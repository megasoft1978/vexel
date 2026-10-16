#define _DARWIN_C_SOURCE 1
#import <Foundation/Foundation.h>

#include "platform/platform_recovery_store.h"

#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static platform_recovery_store_result resolve(const char *root, const char *source,
    char *output, size_t capacity)
{
    return platform_recovery_store_path(bytes(root), bytes(source),
        (base_mut_bytes){(uint8_t *)output, capacity});
}

static bool test_override_identity_and_permissions(void)
{
    char directory[] = "/tmp/vexel-recovery-store-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char source[1024], alias[1024], other[1024], vexel[1024], recovery[1024];
    int n = snprintf(source, sizeof(source), "%s/source.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(source));
    n = snprintf(alias, sizeof(alias), "%s/./source.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(alias));
    n = snprintf(other, sizeof(other), "%s/other.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(other));
    FILE *file = fopen(source, "wb");
    CHECK(file != NULL && fputs("source", file) >= 0 && fclose(file) == 0);
    file = fopen(other, "wb");
    CHECK(file != NULL && fputs("other", file) >= 0 && fclose(file) == 0);
    char first[PLATFORM_RECOVERY_STORE_PATH_MAX];
    char second[PLATFORM_RECOVERY_STORE_PATH_MAX];
    platform_recovery_store_result result = resolve(directory, source, first, sizeof(first));
    CHECK(result.error == PLATFORM_RECOVERY_STORE_OK &&
        result.path_length == strlen(first) && strstr(first, "/Vexel/Recovery/") != NULL &&
        strstr(first + strlen(first) - strlen(".vxrecover"), ".vxrecover") != NULL);
    result = resolve(directory, alias, second, sizeof(second));
    CHECK(result.error == PLATFORM_RECOVERY_STORE_OK && strcmp(first, second) == 0);
    result = resolve(directory, other, second, sizeof(second));
    CHECK(result.error == PLATFORM_RECOVERY_STORE_OK && strcmp(first, second) != 0);
    n = snprintf(vexel, sizeof(vexel), "%s/Vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(vexel));
    n = snprintf(recovery, sizeof(recovery), "%s/Recovery", vexel);
    CHECK(n > 0 && (size_t)n < sizeof(recovery));
    struct stat status;
    CHECK(stat(vexel, &status) == 0 && (status.st_mode & 0777) == 0700);
    CHECK(stat(recovery, &status) == 0 && (status.st_mode & 0777) == 0700);
    char too_small[8];
    result = resolve(directory, source, too_small, sizeof(too_small));
    CHECK(result.error == PLATFORM_RECOVERY_STORE_PATH_LIMIT && too_small[0] == '\0');
    CHECK(unlink(source) == 0 && unlink(other) == 0 &&
        rmdir(recovery) == 0 && rmdir(vexel) == 0 && rmdir(directory) == 0);
    return true;
}

static bool test_invalid_and_unsafe_paths(void)
{
    char directory[] = "/tmp/vexel-recovery-unsafe-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char source[1024], missing[1024], child[1024], recovery[1024], link[1024];
    int n = snprintf(source, sizeof(source), "%s/source.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(source));
    FILE *file = fopen(source, "wb");
    CHECK(file != NULL && fputs("x", file) >= 0 && fclose(file) == 0);
    n = snprintf(missing, sizeof(missing), "%s/missing", directory);
    CHECK(n > 0 && (size_t)n < sizeof(missing));
    char output[PLATFORM_RECOVERY_STORE_PATH_MAX];
    CHECK(resolve(directory, missing, output, sizeof(output)).error ==
        PLATFORM_RECOVERY_STORE_SOURCE_NOT_FOUND);
    CHECK(resolve(directory, directory, output, sizeof(output)).error ==
        PLATFORM_RECOVERY_STORE_SOURCE_NOT_REGULAR);
    n = snprintf(child, sizeof(child), "%s/Vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(child) && mkdir(child, 0700) == 0);
    n = snprintf(recovery, sizeof(recovery), "%s/Recovery", child);
    CHECK(n > 0 && (size_t)n < sizeof(recovery) && mkdir(recovery, 0750) == 0);
    CHECK(resolve(directory, source, output, sizeof(output)).error ==
        PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    CHECK(rmdir(recovery) == 0);
    n = snprintf(link, sizeof(link), "%s/Recovery", child);
    CHECK(n > 0 && (size_t)n < sizeof(link) && symlink(directory, link) == 0);
    CHECK(resolve(directory, source, output, sizeof(output)).error ==
        PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    CHECK(unlink(link) == 0 && rmdir(child) == 0 && unlink(source) == 0 &&
        rmdir(directory) == 0);
    return true;
}

static bool test_default_application_support_location(void)
{
    NSArray<NSString *> *roots = NSSearchPathForDirectoriesInDomains(
        NSApplicationSupportDirectory, NSUserDomainMask, YES);
    CHECK(roots.count > 0U);
    NSString *root = roots[0];
    NSString *vexel_path = [root stringByAppendingPathComponent:@"Vexel"];
    NSString *recovery_path = [vexel_path stringByAppendingPathComponent:@"Recovery"];
    BOOL had_vexel = [[NSFileManager defaultManager] fileExistsAtPath:vexel_path];
    BOOL had_recovery = [[NSFileManager defaultManager] fileExistsAtPath:recovery_path];
    char directory[] = "/tmp/vexel-recovery-default-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char source[1024];
    int n = snprintf(source, sizeof(source), "%s/source.vexel", directory);
    CHECK(n > 0 && (size_t)n < sizeof(source));
    FILE *file = fopen(source, "wb");
    CHECK(file != NULL && fputs("x", file) >= 0 && fclose(file) == 0);
    char output[PLATFORM_RECOVERY_STORE_PATH_MAX];
    platform_recovery_store_result result = platform_recovery_store_path(
        (base_bytes){NULL, 0U}, bytes(source), (base_mut_bytes){
            (uint8_t *)output, sizeof(output)});
    NSString *actual = result.error == PLATFORM_RECOVERY_STORE_OK ?
        [NSString stringWithUTF8String:output] : nil;
    CHECK(result.error == PLATFORM_RECOVERY_STORE_OK &&
        [actual hasPrefix:[recovery_path stringByAppendingString:@"/"]]);
    CHECK(unlink(source) == 0 && rmdir(directory) == 0);
    if (!had_recovery) (void)rmdir(recovery_path.fileSystemRepresentation);
    if (!had_vexel) (void)rmdir(vexel_path.fileSystemRepresentation);
    return true;
}

int main(void)
{
    @autoreleasepool {
        if (!test_override_identity_and_permissions() ||
            !test_invalid_and_unsafe_paths() ||
            !test_default_application_support_location()) return 1;
    }
    (void)puts("platform recovery store: canonical keys and private directories passed");
    return 0;
}
