#ifndef VEXEL_DOCUMENT_SOURCE_SAVE_H
#define VEXEL_DOCUMENT_SOURCE_SAVE_H

#include "platform_atomic_file.h"
#include "xml_tokenizer.h"

typedef enum source_save_error {
    SOURCE_SAVE_OK = 0,
    SOURCE_SAVE_INVALID,
    SOURCE_SAVE_INPUT_LIMIT,
    SOURCE_SAVE_PATH_LIMIT,
    SOURCE_SAVE_TOKENIZER,
    SOURCE_SAVE_DESTINATION,
    SOURCE_SAVE_DURABILITY
} source_save_error;

typedef struct source_save_result {
    source_save_error error;
    svg_xml_diagnostic parse_diagnostic;
    platform_atomic_file_result file_result;
} source_save_result;

/* Source, path and tokenizer storage are borrowed and remain caller-owned.
 * Tokenization must succeed before the destination adapter is called. */
source_save_result source_save_noop(base_bytes source, base_bytes path,
                                    svg_xml_token *tokens, size_t token_capacity,
                                    const platform_atomic_file_hooks *hooks);
const char *source_save_error_name(source_save_error error);

#endif
