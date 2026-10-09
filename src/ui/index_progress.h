#ifndef VEXEL_UI_INDEX_PROGRESS_H
#define VEXEL_UI_INDEX_PROGRESS_H

#include "index.h"

typedef struct ui_index_progress {
    uint64_t generation;
    unsigned percent;
    bool loading;
    bool preview_visible;
    bool failed;
} ui_index_progress;

/* Owner-thread value update; stale generations are ignored. */
bool ui_index_progress_update(ui_index_progress *state,
                              const svg_index_progress *progress);

#endif
