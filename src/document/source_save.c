#include "source_save.h"

#include <string.h>

source_save_result source_save_noop(base_bytes source, base_bytes path,
                                    svg_xml_token *tokens, size_t token_capacity,
                                    const platform_atomic_file_hooks *hooks)
{
    source_save_result result;
    memset(&result, 0, sizeof(result));
    result.error = SOURCE_SAVE_INVALID;
    if (!base_bytes_valid(source) || !base_bytes_valid(path) || path.len == 0 ||
        (token_capacity > 0 && tokens == NULL)) return result;
    if (path.len > PLATFORM_ATOMIC_FILE_MAX_PATH_BYTES) {
        result.error = SOURCE_SAVE_PATH_LIMIT;
        return result;
    }

    svg_xml_options options = svg_xml_default_options();
    if (source.len > options.limits.input_bytes) {
        result.error = SOURCE_SAVE_INPUT_LIMIT;
        return result;
    }
    svg_xml_result parsed = svg_xml_tokenize(source, options, tokens, token_capacity);
    result.parse_diagnostic = parsed.diagnostic;
    if (parsed.diagnostic.category != SVG_XML_ERROR_NONE) {
        result.error = SOURCE_SAVE_TOKENIZER;
        return result;
    }

    result.file_result = platform_atomic_file_replace(path, source, hooks);
    if (result.file_result.error == PLATFORM_ATOMIC_FILE_OK) result.error = SOURCE_SAVE_OK;
    else if (result.file_result.error == PLATFORM_ATOMIC_FILE_DURABILITY)
        result.error = SOURCE_SAVE_DURABILITY;
    else result.error = SOURCE_SAVE_DESTINATION;
    return result;
}

const char *source_save_error_name(source_save_error error)
{
    switch (error) {
        case SOURCE_SAVE_OK: return "ok";
        case SOURCE_SAVE_INVALID: return "invalid";
        case SOURCE_SAVE_INPUT_LIMIT: return "input_limit";
        case SOURCE_SAVE_PATH_LIMIT: return "path_limit";
        case SOURCE_SAVE_TOKENIZER: return "tokenizer";
        case SOURCE_SAVE_DESTINATION: return "destination";
        case SOURCE_SAVE_DURABILITY: return "durability";
        default: return "unknown";
    }
}
