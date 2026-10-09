#include "index_progress.h"

bool ui_index_progress_update(ui_index_progress *state,
                              const svg_index_progress *progress)
{
    if (state == NULL || progress == NULL || progress->source_bytes == 0U) return false;
    if (state->generation > progress->generation) return false;
    state->generation = progress->generation;
    size_t percent = progress->bytes_read >= progress->source_bytes ? 100U :
                     (progress->bytes_read * 100U) / progress->source_bytes;
    state->percent = (unsigned)percent;
    state->loading = !progress->complete && !progress->canceled &&
                     progress->diagnostic.category == SVG_XML_ERROR_NONE;
    state->preview_visible = progress->first_preview_ready && !progress->canceled;
    state->failed = progress->diagnostic.category != SVG_XML_ERROR_NONE;
    return true;
}
