#ifndef VEXEL_XML_TOKENIZER_PRIVATE_H
#define VEXEL_XML_TOKENIZER_PRIVATE_H

#include "xml_tokenizer.h"

#define XML_MAX_INPUT_BYTES (16U * 1024U * 1024U)
#define XML_MAX_DEPTH 256U
#define XML_MAX_NODES 1000000U
#define XML_MAX_ATTRIBUTES_PER_ELEMENT 4096U
#define XML_MAX_TOTAL_ATTRIBUTES 1000000U
#define XML_MAX_TEXT_RUN_BYTES (4U * 1024U * 1024U)

svg_xml_result svg_xml_tokenizer_run(base_bytes input, svg_xml_options options,
    svg_xml_token *tokens, size_t token_capacity, svg_xml_cancel_check_fn canceled,
    void *cancel_context);

#endif
