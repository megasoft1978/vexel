#define _DARWIN_C_SOURCE 1
#import <Foundation/Foundation.h>
#import <CommonCrypto/CommonDigest.h>

#include "platform_recovery_store.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static platform_recovery_store_result store_result(
    platform_recovery_store_error error)
{
    return (platform_recovery_store_result){.error = error};
}

static bool valid_slice(base_bytes bytes)
{
    return bytes.ptr != NULL && bytes.len > 0U &&
        memchr(bytes.ptr, '\0', bytes.len) == NULL;
}

static bool directory_open_or_create(int parent_fd, const char *name, int *out_fd)
{
    bool created = mkdirat(parent_fd, name, 0700) == 0;
    if (!created && errno != EEXIST) return false;
    int fd = openat(parent_fd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return false;
    struct stat status;
    bool safe = (!created || fchmod(fd, 0700) == 0) && fstat(fd, &status) == 0 &&
        S_ISDIR(status.st_mode) &&
        status.st_uid == geteuid() && (status.st_mode & 0077) == 0;
    if (!safe) {
        (void)close(fd);
        return false;
    }
    *out_fd = fd;
    return true;
}

static NSString *default_support_root(void)
{
    NSArray<NSString *> *paths = NSSearchPathForDirectoriesInDomains(
        NSApplicationSupportDirectory, NSUserDomainMask, YES);
    if (paths.count == 0U) return nil;
    return paths[0];
}

platform_recovery_store_result platform_recovery_store_path(
    base_bytes application_support_root, base_bytes source_path,
    base_mut_bytes out_path)
{
    if (!valid_slice(source_path) ||
        (application_support_root.len > 0U && !valid_slice(application_support_root)) ||
        out_path.ptr == NULL || out_path.len == 0U)
        return store_result(PLATFORM_RECOVERY_STORE_INVALID_ARGUMENT);
    out_path.ptr[0] = '\0';
    char source_input[PLATFORM_RECOVERY_STORE_PATH_MAX];
    if (source_path.len >= sizeof(source_input))
        return store_result(PLATFORM_RECOVERY_STORE_PATH_LIMIT);
    memcpy(source_input, source_path.ptr, source_path.len);
    source_input[source_path.len] = '\0';
    char canonical_source[PLATFORM_RECOVERY_STORE_PATH_MAX];
    if (realpath(source_input, canonical_source) == NULL)
        return store_result(errno == ENOENT ? PLATFORM_RECOVERY_STORE_SOURCE_NOT_FOUND :
            PLATFORM_RECOVERY_STORE_IO);
    struct stat source_status;
    if (lstat(canonical_source, &source_status) != 0)
        return store_result(PLATFORM_RECOVERY_STORE_IO);
    if (!S_ISREG(source_status.st_mode))
        return store_result(PLATFORM_RECOVERY_STORE_SOURCE_NOT_REGULAR);

    NSString *support_root = nil;
    if (application_support_root.len == 0U) support_root = default_support_root();
    else support_root = [[NSString alloc] initWithBytes:application_support_root.ptr
        length:application_support_root.len encoding:NSUTF8StringEncoding];
    if (support_root == nil || !support_root.isAbsolutePath)
        return store_result(PLATFORM_RECOVERY_STORE_INVALID_ARGUMENT);
    NSString *root_path = [support_root stringByStandardizingPath];
    const char *root_utf8 = root_path.fileSystemRepresentation;
    if (root_utf8 == NULL || strlen(root_utf8) >= PLATFORM_RECOVERY_STORE_PATH_MAX)
        return store_result(PLATFORM_RECOVERY_STORE_PATH_LIMIT);
    struct stat root_status;
    if (lstat(root_utf8, &root_status) != 0 || !S_ISDIR(root_status.st_mode))
        return store_result(PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    int root_fd = open(root_utf8, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (root_fd < 0) return store_result(PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    int vexel_fd = -1;
    if (!directory_open_or_create(root_fd, "Vexel", &vexel_fd)) {
        (void)close(root_fd);
        return store_result(PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    }
    int recovery_fd = -1;
    bool directory_ready = directory_open_or_create(vexel_fd, "Recovery", &recovery_fd);
    (void)close(vexel_fd);
    (void)close(root_fd);
    if (!directory_ready)
        return store_result(PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE);
    (void)close(recovery_fd);

    uint8_t digest[CC_SHA256_DIGEST_LENGTH];
    size_t canonical_length = strlen(canonical_source);
    if (canonical_length > UINT32_MAX ||
        CC_SHA256(canonical_source, (CC_LONG)canonical_length, digest) == NULL)
        return store_result(PLATFORM_RECOVERY_STORE_IO);
    char filename[sizeof(digest) * 2U + sizeof(".vxrecover")];
    static const char hex[] = "0123456789abcdef";
    for (size_t i = 0U; i < sizeof(digest); ++i) {
        filename[i * 2U] = hex[digest[i] >> 4U];
        filename[i * 2U + 1U] = hex[digest[i] & 0x0fU];
    }
    memcpy(filename + sizeof(digest) * 2U, ".vxrecover", sizeof(".vxrecover"));
    int amount = snprintf((char *)out_path.ptr, out_path.len, "%s/Vexel/Recovery/%s",
        root_utf8, filename);
    if (amount < 0 || (size_t)amount >= out_path.len ||
        (size_t)amount >= PLATFORM_RECOVERY_STORE_PATH_MAX) {
        out_path.ptr[0] = '\0';
        return store_result(PLATFORM_RECOVERY_STORE_PATH_LIMIT);
    }
    platform_recovery_store_result result = store_result(PLATFORM_RECOVERY_STORE_OK);
    result.path_length = (size_t)amount;
    return result;
}

const char *platform_recovery_store_error_name(platform_recovery_store_error error)
{
    switch (error) {
        case PLATFORM_RECOVERY_STORE_OK: return "ok";
        case PLATFORM_RECOVERY_STORE_INVALID_ARGUMENT: return "invalid_argument";
        case PLATFORM_RECOVERY_STORE_SOURCE_NOT_FOUND: return "source_not_found";
        case PLATFORM_RECOVERY_STORE_SOURCE_NOT_REGULAR: return "source_not_regular";
        case PLATFORM_RECOVERY_STORE_DIRECTORY_UNSAFE: return "directory_unsafe";
        case PLATFORM_RECOVERY_STORE_PATH_LIMIT: return "path_limit";
        case PLATFORM_RECOVERY_STORE_IO: return "io";
    }
    return "unknown";
}
