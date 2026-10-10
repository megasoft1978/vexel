#ifndef VEXEL_UI_ICON_PATHS_H
#define VEXEL_UI_ICON_PATHS_H

#include "ui_core.h"

typedef struct ui_icon_definition {
    ui_icon_kind icon;
    const char *name;
    const char *path;
    size_t path_length;
} ui_icon_definition;

const ui_icon_definition *ui_icon_definitions(size_t *out_count);

#endif
