#include "source_save.h"
#include "platform_atomic_new_file.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static const uint8_t fixture[] =
    "<?xml version='1.0' encoding=\"UTF-8\"?>\r\n"
    "<!-- authored preservation fixture -->\n"
    "<svg xmlns='http://www.w3.org/2000/svg' xmlns:x='urn:x' data-q=\"a'b\">\r"
    "  <metadata>  keep  whitespace\r\ninside </metadata>\n"
    "  <x:unknown x:quoted='single' note=\"double\"/>\r\n"
    "</svg>\n";

struct injected {
    bool fail_write;
    bool fail_rename;
    bool short_write;
    unsigned calls;
};

static ssize_t test_write(int fd, const void *bytes, size_t length, void *context)
{
    struct injected *state = context;
    state->calls++;
    if (state->fail_write) {
        state->fail_write = false;
        errno = EIO;
        return -1;
    }
    if (state->short_write && length > 3) length = 3;
    return write(fd, bytes, length);
}

static int test_rename(const char *from, const char *to, void *context)
{
    struct injected *state = context;
    if (state->fail_rename) {
        state->fail_rename = false;
        errno = EIO;
        return -1;
    }
    return rename(from, to);
}

static bool write_file(const char *path, const uint8_t *bytes, size_t length, mode_t mode)
{
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, mode);
    if (fd < 0) return false;
    size_t offset = 0;
    while (offset < length) {
        ssize_t count = write(fd, bytes + offset, length - offset);
        if (count <= 0) { (void)close(fd); return false; }
        offset += (size_t)count;
    }
    return fchmod(fd, mode) == 0 && close(fd) == 0;
}

static bool target_is(const char *path, const uint8_t *expected, size_t length, mode_t mode)
{
    struct stat info;
    if (stat(path, &info) != 0 || (info.st_mode & 07777) != mode ||
        (size_t)info.st_size != length) return false;
    int fd = open(path, O_RDONLY);
    if (fd < 0) return false;
    uint8_t buffer[4096];
    if (length > sizeof(buffer)) { (void)close(fd); return false; }
    ssize_t count = read(fd, buffer, sizeof(buffer));
    bool same = count >= 0 && (size_t)count == length && memcmp(buffer, expected, length) == 0;
    return close(fd) == 0 && same;
}

static size_t temp_count(const char *directory)
{
    DIR *dir = opendir(directory);
    if (dir == NULL) return SIZE_MAX;
    size_t count = 0;
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL)
        if (strncmp(entry->d_name, ".vexel-save-", 12) == 0 ||
            strncmp(entry->d_name, ".vexel-new-", 11) == 0) count++;
    (void)closedir(dir);
    return count;
}

static bool test_save_and_failures(const char *directory)
{
    char target[1024];
    int written = snprintf(target, sizeof(target), "%s/document.svg", directory);
    CHECK(written > 0 && (size_t)written < sizeof(target));
    static const uint8_t old_bytes[] = "<svg>original</svg>";
    const mode_t mode = 0640;
    CHECK(write_file(target, old_bytes, sizeof(old_bytes) - 1, mode));
    svg_xml_token tokens[128];
    base_bytes source = {fixture, sizeof(fixture) - 1};
    base_bytes path = {(const uint8_t *)target, strlen(target)};
    struct injected state = {.short_write = true};
    platform_atomic_file_hooks hooks = {test_write, test_rename, &state};

    source_save_result saved = source_save_noop(source, path, tokens, 128, &hooks);
    CHECK(saved.error == SOURCE_SAVE_OK);
    CHECK(saved.file_result.file_fsynced && saved.file_result.renamed &&
          saved.file_result.directory_fsynced);
    (void)puts("native_save file_fsync=true rename=true parent_directory_fsync=true");
    CHECK(state.calls > 1);
    CHECK(target_is(target, fixture, sizeof(fixture) - 1, mode));
    CHECK(temp_count(directory) == 0);

    CHECK(write_file(target, old_bytes, sizeof(old_bytes) - 1, mode));
    static const uint8_t malformed[] = "<svg><broken></svg>";
    base_bytes bad = {malformed, sizeof(malformed) - 1};
    saved = source_save_noop(bad, path, tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_TOKENIZER);
    CHECK(target_is(target, old_bytes, sizeof(old_bytes) - 1, mode));
    CHECK(temp_count(directory) == 0);

    state = (struct injected){.fail_write = true};
    saved = source_save_noop(source, path, tokens, 128, &hooks);
    CHECK(saved.error == SOURCE_SAVE_DESTINATION);
    CHECK(target_is(target, old_bytes, sizeof(old_bytes) - 1, mode));
    CHECK(temp_count(directory) == 0);

    state = (struct injected){.fail_rename = true};
    saved = source_save_noop(source, path, tokens, 128, &hooks);
    CHECK(saved.error == SOURCE_SAVE_DESTINATION);
    CHECK(target_is(target, old_bytes, sizeof(old_bytes) - 1, mode));
    CHECK(temp_count(directory) == 0);

    base_bytes empty_path = {NULL, 0};
    saved = source_save_noop(source, empty_path, tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_INVALID);
    uint8_t too_long[4096];
    memset(too_long, 'x', sizeof(too_long));
    base_bytes long_path = {too_long, sizeof(too_long)};
    saved = source_save_noop(source, long_path, tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_PATH_LIMIT);
    char missing_target[1024];
    written = snprintf(missing_target, sizeof(missing_target), "%s/missing.svg", directory);
    CHECK(written > 0 && (size_t)written < sizeof(missing_target));
    base_bytes missing_path = {(const uint8_t *)missing_target, strlen(missing_target)};
    saved = source_save_noop(source, missing_path, tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_OK && saved.file_result.renamed &&
          saved.file_result.file_fsynced && saved.file_result.directory_fsynced);
    CHECK(target_is(missing_target, fixture, sizeof(fixture) - 1U, 0600));
    platform_atomic_new_file_result duplicate = platform_atomic_file_create_new(
        missing_path, (base_bytes){old_bytes, sizeof(old_bytes) - 1U}, NULL);
    CHECK(duplicate.error == PLATFORM_ATOMIC_NEW_FILE_EXISTS && !duplicate.published);
    CHECK(target_is(missing_target, fixture, sizeof(fixture) - 1U, 0600));
    CHECK(temp_count(directory) == 0U);

    char malformed_target[1024];
    written = snprintf(malformed_target, sizeof(malformed_target),
        "%s/malformed.svg", directory);
    CHECK(written > 0 && (size_t)written < sizeof(malformed_target));
    saved = source_save_noop(bad,
        (base_bytes){(const uint8_t *)malformed_target, strlen(malformed_target)},
        tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_TOKENIZER &&
        access(malformed_target, F_OK) != 0 && temp_count(directory) == 0U);

    char failed_target[1024];
    written = snprintf(failed_target, sizeof(failed_target), "%s/failed.svg", directory);
    CHECK(written > 0 && (size_t)written < sizeof(failed_target));
    state = (struct injected){.fail_write = true};
    saved = source_save_noop(source,
        (base_bytes){(const uint8_t *)failed_target, strlen(failed_target)},
        tokens, 128, &hooks);
    CHECK(saved.error == SOURCE_SAVE_DESTINATION && access(failed_target, F_OK) != 0);
    CHECK(temp_count(directory) == 0U);
    base_bytes directory_path = {(const uint8_t *)directory, strlen(directory)};
    saved = source_save_noop(source, directory_path, tokens, 128, NULL);
    CHECK(saved.error == SOURCE_SAVE_DESTINATION);
    CHECK(saved.file_result.error == PLATFORM_ATOMIC_FILE_NOT_REGULAR);
    CHECK(target_is(target, old_bytes, sizeof(old_bytes) - 1, mode));
    CHECK(temp_count(directory) == 0);
    return true;
}

static int save_paths(const char *source_path, const char *target_path)
{
    int fd = open(source_path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 2;
    struct stat info;
    if (fstat(fd, &info) != 0 || !S_ISREG(info.st_mode) || info.st_size < 0 ||
        (uintmax_t)info.st_size > (uintmax_t)(16u * 1024u * 1024u)) {
        (void)close(fd);
        return 2;
    }
    size_t length = (size_t)info.st_size;
    uint8_t *source = length == 0 ? NULL : malloc(length);
    svg_xml_token *tokens = calloc(65536, sizeof(*tokens));
    if ((length != 0 && source == NULL) || tokens == NULL) {
        free(source);
        free(tokens);
        (void)close(fd);
        return 2;
    }
    size_t offset = 0;
    while (offset < length) {
        ssize_t count = read(fd, source + offset, length - offset);
        if (count <= 0) { free(source); free(tokens); (void)close(fd); return 2; }
        offset += (size_t)count;
    }
    if (close(fd) != 0) { free(source); free(tokens); return 2; }
    base_bytes source_bytes = {source, length};
    base_bytes path = {(const uint8_t *)target_path, strlen(target_path)};
    source_save_result result = source_save_noop(source_bytes, path, tokens, 65536, NULL);
    if (result.error != SOURCE_SAVE_OK) {
        (void)fprintf(stderr, "%s\n", source_save_error_name(result.error));
        free(source);
        free(tokens);
        return 1;
    }
    free(source);
    free(tokens);
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 4 && strcmp(argv[1], "--save") == 0)
        return save_paths(argv[2], argv[3]);
    if (argc != 1) return 2;
    char template[] = "/tmp/vexel-source-save-XXXXXX";
    char *directory = mkdtemp(template);
    if (directory == NULL) return 1;
    bool passed = test_save_and_failures(directory);
    char target[1024];
    (void)snprintf(target, sizeof(target), "%s/document.svg", directory);
    char missing_target[1024];
    (void)snprintf(missing_target, sizeof(missing_target), "%s/missing.svg", directory);
    char failed_target[1024];
    (void)snprintf(failed_target, sizeof(failed_target), "%s/failed.svg", directory);
    char malformed_target[1024];
    (void)snprintf(malformed_target, sizeof(malformed_target), "%s/malformed.svg", directory);
    (void)unlink(target);
    (void)unlink(missing_target);
    (void)unlink(failed_target);
    (void)unlink(malformed_target);
    (void)rmdir(directory);
    if (!passed) return 1;
    (void)puts("source-save C tests passed; byte equality, permission preservation, atomic create, no-replace collision, and cleanup verified");
    return 0;
}
