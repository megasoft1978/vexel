#include "viewer_workspace.h"

#include <string.h>

static const viewer_workspace_spec workspace_table[VIEWER_WORKSPACE_COUNT] = {
    {"draw", "Draw", "Object", true, true, false},
    {"nodes", "Nodes", "Geometry", true, true, false},
    {"animate", "Animate", "Object", true, true, true},
    {"inspect", "Inspect", "Document", true, true, false}
};

const viewer_workspace_spec *viewer_workspace_table(void)
{
    return workspace_table;
}

size_t viewer_workspace_count(void)
{
    return (size_t)VIEWER_WORKSPACE_COUNT;
}

const viewer_workspace_spec *viewer_workspace_get(viewer_workspace_kind kind)
{
    if ((size_t)kind >= (size_t)VIEWER_WORKSPACE_COUNT)
        return &workspace_table[VIEWER_WORKSPACE_DRAW];
    return &workspace_table[(size_t)kind];
}

viewer_workspace_kind viewer_workspace_default(void)
{
    return VIEWER_WORKSPACE_DRAW;
}

viewer_workspace_kind viewer_workspace_from_name(const char *name, size_t length)
{
    if (name == NULL || length == 0U) return viewer_workspace_default();
    for (size_t index = 0U; index < (size_t)VIEWER_WORKSPACE_COUNT; ++index) {
        const char *candidate = workspace_table[index].name;
        size_t candidate_length = strlen(candidate);
        if (candidate_length == length && memcmp(candidate, name, length) == 0)
            return (viewer_workspace_kind)index;
    }
    return viewer_workspace_default();
}

const char *viewer_workspace_name(viewer_workspace_kind kind)
{
    return viewer_workspace_get(kind)->name;
}

viewer_workspace_kind viewer_workspace_cycle(viewer_workspace_kind current, bool backwards)
{
    size_t count = (size_t)VIEWER_WORKSPACE_COUNT;
    size_t index = (size_t)current;
    if (index >= count) index = (size_t)viewer_workspace_default();
    index = backwards ? (index + count - 1U) % count : (index + 1U) % count;
    return (viewer_workspace_kind)index;
}
