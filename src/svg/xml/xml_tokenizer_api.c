#include "xml_tokenizer.h"
#include "xml_tokenizer_private.h"

svg_xml_options svg_xml_default_options(void)
{
    svg_xml_options options = {{XML_MAX_INPUT_BYTES, XML_MAX_DEPTH, XML_MAX_NODES,
        XML_MAX_ATTRIBUTES_PER_ELEMENT, XML_MAX_TOTAL_ATTRIBUTES,
        XML_MAX_TEXT_RUN_BYTES}};
    return options;
}

static svg_xml_options bounded_options(svg_xml_options options)
{
    const svg_xml_options maximum = svg_xml_default_options();
    if (options.limits.input_bytes > maximum.limits.input_bytes)
        options.limits.input_bytes = maximum.limits.input_bytes;
    if (options.limits.nesting_depth > maximum.limits.nesting_depth)
        options.limits.nesting_depth = maximum.limits.nesting_depth;
    if (options.limits.nodes > maximum.limits.nodes)
        options.limits.nodes = maximum.limits.nodes;
    if (options.limits.attributes_per_element > maximum.limits.attributes_per_element)
        options.limits.attributes_per_element = maximum.limits.attributes_per_element;
    if (options.limits.total_attributes > maximum.limits.total_attributes)
        options.limits.total_attributes = maximum.limits.total_attributes;
    if (options.limits.text_run_bytes > maximum.limits.text_run_bytes)
        options.limits.text_run_bytes = maximum.limits.text_run_bytes;
    return options;
}

svg_xml_result svg_xml_tokenize(base_bytes input, svg_xml_options options,
                                svg_xml_token *tokens, size_t token_capacity)
{
    return svg_xml_tokenizer_run(input, bounded_options(options), tokens,
        token_capacity, NULL, NULL);
}

svg_xml_result svg_xml_tokenize_large(base_bytes input, svg_xml_token *tokens,
                                      size_t token_capacity)
{
    return svg_xml_tokenize_large_cancelable(input, tokens, token_capacity, NULL, NULL);
}

svg_xml_result svg_xml_tokenize_large_cancelable(base_bytes input, svg_xml_token *tokens,
    size_t token_capacity, svg_xml_cancel_check_fn canceled, void *cancel_context)
{
    svg_xml_options options = svg_xml_default_options();
    options.limits.input_bytes = 1024U * 1024U * 1024U;
    return svg_xml_tokenizer_run(input, options, tokens, token_capacity, canceled,
        cancel_context);
}

const char *svg_xml_error_name(svg_xml_error error)
{
    static const char *const names[] = {"none", "invalid_argument", "input_limit",
        "invalid_utf8", "invalid_character", "unsupported_encoding", "invalid_name",
        "malformed_syntax", "duplicate_attribute", "mismatched_tag", "depth_limit",
        "node_limit", "attribute_limit", "text_limit", "output_exhausted",
        "unsupported_dtd", "multiple_roots", "incomplete_document"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ?
        names[(unsigned)error] : "unknown";
}
