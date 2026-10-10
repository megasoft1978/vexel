#include "mcp_protocol.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint32_t next_random(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value == 0U ? 1U : value;
    return *state;
}

static const char *const candidates[] = {
    "document_open", "document_close", "document_list", "document_info", "tree_get",
    "element_get", "element_query", "attribute_set", "attribute_remove", "text_set",
    "element_add", "element_delete", "element_duplicate", "element_reorder", "element_group",
    "element_ungroup", "transaction", "undo", "redo", "save", "save_as", "render_png",
    "selection_get", "selection_set", "export_svg"
};

static const char *const seeds[] = {
    "{}",
    "{\"documentId\":\"doc-00000001\"}",
    "{\"path\":\"/tmp/example.svg\"}",
    "{\"documentId\":\"d\",\"elementId\":\"r\",\"name\":\"fill\",\"value\":\"red\"}",
    "{\"documentId\":\"d\",\"parentId\":\"@1:1\",\"template\":\"rect\",\"attributes\":{\"x\":\"1\"}}",
    "{\"documentId\":\"d\",\"edits\":[{\"op\":\"text_set\",\"elementId\":\"t\",\"text\":\"ok\"}]}"
};

int main(int argc, char **argv)
{
    uint64_t requested = 200000U;
    uint32_t seed = 12648430U;
    if (argc > 1) {
        char *end = NULL;
        unsigned long long value = strtoull(argv[1], &end, 10);
        if (end == argv[1] || *end != '\0' || value == 0U || value > 1000000U) return EXIT_FAILURE;
        requested = (uint64_t)value;
    }
    if (argc > 2) {
        char *end = NULL;
        unsigned long value = strtoul(argv[2], &end, 10);
        if (end == argv[2] || *end != '\0' || value > UINT32_MAX) return EXIT_FAILURE;
        seed = (uint32_t)value;
    }
    if (argc > 3) return EXIT_FAILURE;
    uint32_t state = seed == 0U ? 1U : seed;
    uint8_t bytes[512];
    size_t parsed_count = 0U;
    size_t validated_count = 0U;
    for (uint64_t trial = 0U; trial < requested; trial++) {
        size_t length = 0U;
        if ((trial & 3U) == 0U) {
            const char *snippet = seeds[next_random(&state) % (sizeof(seeds) / sizeof(seeds[0]))];
            length = strlen(snippet);
            memcpy(bytes, snippet, length);
        } else {
            length = next_random(&state) % sizeof(bytes);
            for (size_t index = 0U; index < length; index++) bytes[index] = (uint8_t)next_random(&state);
        }
        mcp_json_doc doc = {0};
        if (!mcp_json_parse(bytes, length, &doc)) continue;
        parsed_count++;
        const char *error = NULL;
        const char *tool = candidates[trial % (sizeof(candidates) / sizeof(candidates[0]))];
        (void)mcp_tool_validate(&doc, doc.root, tool, &error);
        validated_count++;
        if ((trial % 32U) == 0U) {
            mcp_protocol_session session = {0};
            mcp_protocol_backend backend = {0};
            mcp_json_buffer output = {0};
            if (!mcp_json_buffer_init(&output, 64U, MCP_JSON_MAX_BYTES)) {
                mcp_json_doc_destroy(&doc);
                return EXIT_FAILURE;
            }
            (void)mcp_protocol_handle(&session, bytes, length, &backend, &output);
            mcp_json_buffer_destroy(&output);
        }
        mcp_json_doc_destroy(&doc);
    }
    (void)printf("mcp-fuzz: cases=%llu parsed=%zu argument-validations=%zu seed=%u\n",
        (unsigned long long)requested, parsed_count, validated_count, seed);
    return EXIT_SUCCESS;
}
