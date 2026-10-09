#include "index_progress.h"

bool ui_index_progress_update(ui_index_progress *state,
                              const svg_index_progress *progress)
{
    if (state == NULL || progress == NULL || progress->source_bytes == 0U) return false;
    if (state->generation > progress->generation) return false;
    if (state->generation != progress->generation) state->preview_visible = false;
    state->generation = progress->generation;
    size_t percent = progress->bytes_read >= progress->source_bytes ? 100U :
                     (progress->bytes_read * 100U) / progress->source_bytes;
    state->percent = (unsigned)percent;
    state->loading = !progress->complete && !progress->canceled &&
                     progress->diagnostic.category == SVG_XML_ERROR_NONE;
    state->failed = progress->diagnostic.category != SVG_XML_ERROR_NONE;
    if (progress->canceled || state->failed) state->preview_visible = false;
    return true;
}

bool ui_index_progress_publish_preview(ui_index_progress *state, uint64_t generation)
{
    if (state == NULL || state->generation != generation || state->failed) return false;
    state->preview_visible = true;
    return true;
}
