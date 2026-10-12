#ifndef VEXEL_APP_VIEWER_WORKSPACE_H
#define VEXEL_APP_VIEWER_WORKSPACE_H

#include <stdbool.h>
#include <stddef.h>

/* Task workspaces follow Blender's topbar tabs: each names a task, not a mode.
 * A workspace only decides which panes are visible and which inspector group is
 * active, so switching never changes the editing model or the document. */
typedef enum viewer_workspace_kind {
    VIEWER_WORKSPACE_DRAW = 0,
    VIEWER_WORKSPACE_NODES,
    VIEWER_WORKSPACE_ANIMATE,
    VIEWER_WORKSPACE_INSPECT,
    VIEWER_WORKSPACE_COUNT
} viewer_workspace_kind;

typedef struct viewer_workspace_spec {
    const char *name;          /* Stable identifier stored in settings. */
    const char *label;         /* User-visible tab label. */
    const char *inspector_section; /* Inspector section this workspace expands. */
    bool structure;            /* Layers/structure panel visible. */
    bool inspector;            /* Inspector panel visible. */
    bool timeline;             /* Animation timeline visible. */
} viewer_workspace_spec;

const viewer_workspace_spec *viewer_workspace_table(void);
size_t viewer_workspace_count(void);
const viewer_workspace_spec *viewer_workspace_get(viewer_workspace_kind kind);
viewer_workspace_kind viewer_workspace_default(void);
/* An unknown or empty name falls back to the default workspace. */
viewer_workspace_kind viewer_workspace_from_name(const char *name, size_t length);
const char *viewer_workspace_name(viewer_workspace_kind kind);
/* Wraps in both directions; an out-of-range kind is treated as the default. */
viewer_workspace_kind viewer_workspace_cycle(viewer_workspace_kind current, bool backwards);
/* The keyboard hint shown for cycling workspaces. */
const char *viewer_workspace_shortcut_hint(void);

#endif
