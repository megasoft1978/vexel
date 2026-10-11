#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "viewer_internal.h"

#include <sys/stat.h>

bool viewer_file_stamp_read(const char *path, viewer_file_stamp *out)
{
    if (path == NULL || out == NULL) return false;
    struct stat status;
    if (stat(path, &status) != 0 || !S_ISREG(status.st_mode)) return false;
    out->size = status.st_size;
    out->modified = status.st_mtimespec;
    out->valid = true;
    return true;
}

bool viewer_file_stamp_equal(const viewer_file_stamp *left,
                             const viewer_file_stamp *right)
{
    return left != NULL && right != NULL && left->valid && right->valid &&
        left->size == right->size &&
        left->modified.tv_sec == right->modified.tv_sec &&
        left->modified.tv_nsec == right->modified.tv_nsec;
}

void viewer_update_file_stamp(viewer_app *app)
{
    if (app == NULL) return;
    app->file_stamp = (viewer_file_stamp){0};
    if (app->path[0] != '\0') (void)viewer_file_stamp_read(app->path, &app->file_stamp);
    app->external_change_pending = false;
}

bool viewer_file_changed_on_disk(viewer_app *app)
{
    if (app == NULL) return false;
    if (app->external_change_pending) return true;
    viewer_file_stamp current = {0};
    (void)viewer_file_stamp_read(app->path, &current);
    bool changed = current.valid != app->file_stamp.valid ||
        (current.valid && !viewer_file_stamp_equal(&current, &app->file_stamp));
    if (changed) app->external_change_pending = true;
    return changed;
}
