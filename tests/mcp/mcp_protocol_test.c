#include "mcp_protocol.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(value) do { if (!(value)) { \
    (void)fprintf(stderr, "mcp protocol check failed at line %d: %s\n", __LINE__, #value); \
    return EXIT_FAILURE; \
} } while (0)

static bool fake_tool(void *context, const mcp_json_doc *request, size_t args,
                      const char *name, mcp_tool_response *out)
{
    (void)context;
    (void)request;
    (void)args;
    return mcp_json_buffer_literal(&out->structured_json,
        strcmp(name, "document_list") == 0 ? "{\"documents\":[]}" : "{\"ok\":true}");
}

static bool fake_resource(void *context, const mcp_json_doc *request,
                          const char *uri, mcp_json_buffer *out)
{
    (void)context;
    (void)request;
    if (strcmp(uri, "vexel://documents/current/source") != 0) return false;
    return mcp_json_buffer_literal(out,
        "[{\"uri\":\"vexel://documents/current/source\",\"mimeType\":\"image/svg+xml\",\"text\":\"<svg/>\"}]");
}

static bool handle(mcp_protocol_session *session, const char *line, mcp_json_buffer *out)
{
    mcp_protocol_backend backend = {fake_tool, fake_resource, NULL};
    return mcp_protocol_handle(session, (const uint8_t *)line, strlen(line), &backend, out);
}

int main(void)
{
    mcp_json_doc doc = {0};
    const uint8_t valid[] = "{\"a\":[true,false,null,12],\"s\":\"x\\uD83D\\uDE00\\n\"}";
    CHECK(mcp_json_parse(valid, sizeof(valid) - 1U, &doc));
    size_t a = mcp_json_object_get(&doc, doc.root, "a");
    CHECK(a != MCP_JSON_NO_TOKEN && doc.tokens[a].type == MCP_JSON_ARRAY);
    CHECK(mcp_json_array_get(&doc, a, 0U) != MCP_JSON_NO_TOKEN);
    uint64_t number = 0U;
    CHECK(mcp_json_uint(&doc, mcp_json_array_get(&doc, a, 3U), &number) && number == 12U);
    char decoded[32];
    size_t decoded_length = 0U;
    CHECK(mcp_json_string_copy(&doc, mcp_json_object_get(&doc, doc.root, "s"),
                               decoded, sizeof(decoded), &decoded_length));
    CHECK(decoded_length == 6U && memcmp(decoded, "x\xf0\x9f\x98\x80\n", 6U) == 0);
    mcp_json_doc_destroy(&doc);

    const char *invalid[] = {"", "{", "[1,]", "{\"a\" 1}", "01", "1e", "\"\\uD800\"",
        "\"\\uDC00\"", "\"\\x\"", "true false", "[NaN]"};
    for (size_t index = 0U; index < sizeof(invalid) / sizeof(invalid[0]); index++)
        CHECK(!mcp_json_parse((const uint8_t *)invalid[index], strlen(invalid[index]), &doc));
    uint8_t invalid_utf8[] = {'"', 0xc0U, 0xafU, '"'};
    CHECK(!mcp_json_parse(invalid_utf8, sizeof(invalid_utf8), &doc));
    char deep[MCP_JSON_MAX_DEPTH * 2U + 8U];
    size_t deep_length = 0U;
    for (size_t index = 0U; index < MCP_JSON_MAX_DEPTH + 1U; index++) deep[deep_length++] = '[';
    deep[deep_length++] = '0';
    for (size_t index = 0U; index < MCP_JSON_MAX_DEPTH + 1U; index++) deep[deep_length++] = ']';
    CHECK(!mcp_json_parse((const uint8_t *)deep, deep_length, &doc));

    mcp_json_buffer tools = {0};
    CHECK(mcp_json_buffer_init(&tools, 1024U, MCP_JSON_MAX_BYTES));
    CHECK(mcp_tools_list_json(&tools));
    CHECK(mcp_json_parse((const uint8_t *)tools.data, tools.length, &doc));
    CHECK(doc.tokens[doc.root].type == MCP_JSON_ARRAY && doc.tokens[doc.root].child_count >= 20U);
    mcp_json_doc_destroy(&doc);
    mcp_json_buffer_destroy(&tools);

    const char *good_args = "{\"documentId\":\"doc\",\"elementId\":\"rect\",\"name\":\"fill\",\"value\":\"red\"}";
    CHECK(mcp_json_parse((const uint8_t *)good_args, strlen(good_args), &doc));
    const char *validation_error = NULL;
    CHECK(mcp_tool_validate(&doc, doc.root, "attribute_set", &validation_error));
    mcp_json_doc_destroy(&doc);
    const char *bad_args = "{\"documentId\":\"doc\",\"elementId\":\"rect\",\"name\":4,\"value\":\"red\",\"extra\":true}";
    CHECK(mcp_json_parse((const uint8_t *)bad_args, strlen(bad_args), &doc));
    CHECK(!mcp_tool_validate(&doc, doc.root, "attribute_set", &validation_error));
    mcp_json_doc_destroy(&doc);

    mcp_json_buffer response = {0};
    CHECK(mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES));
    mcp_protocol_session legacy = {0};
    CHECK(handle(&legacy,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"}}}", &response));
    CHECK(strstr(response.data, "\"protocolVersion\":\"2025-11-25\"") != NULL);
    response.length = 0U;
    CHECK(handle(&legacy, "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}", &response));
    CHECK(response.length == 0U && legacy.initialized);
    response.length = 0U;
    CHECK(handle(&legacy, "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"params\":{}}", &response));
    CHECK(strstr(response.data, "document_open") != NULL && strstr(response.data, "additionalProperties") != NULL);
    response.length = 0U;
    CHECK(handle(&legacy,
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"document_list\",\"arguments\":{}}}", &response));
    CHECK(strstr(response.data, "structuredContent") != NULL && strstr(response.data, "documents") != NULL);
    response.length = 0U;
    CHECK(handle(&legacy,
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"not_a_tool\",\"arguments\":{}}}", &response));
    CHECK(strstr(response.data, "unknown_tool") != NULL && strstr(response.data, "isError\":true") != NULL);
    response.length = 0U;
    CHECK(handle(&legacy, "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/read\",\"params\":{\"uri\":\"vexel://documents/current/source\"}}", &response));
    CHECK(strstr(response.data, "<svg/>") != NULL);
    response.length = 0U;
    CHECK(handle(&legacy, "{bad}", &response));
    CHECK(strstr(response.data, "-32700") != NULL);

    mcp_protocol_session modern = {0};
    response.length = 0U;
    CHECK(handle(&modern,
        "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"server/discover\",\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{}}}}",
        &response));
    CHECK(strstr(response.data, "2026-07-28") != NULL && modern.modern);
    response.length = 0U;
    CHECK(handle(&modern,
        "{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"ping\",\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\"}}}",
        &response));
    CHECK(strstr(response.data, "serverInfo") != NULL);
    response.length = 0U;
    CHECK(handle(&modern, "{\"jsonrpc\":\"2.0\",\"id\":12,\"method\":\"ping\",\"params\":{}}", &response));
    CHECK(strstr(response.data, "-32022") != NULL);
    CHECK(!mcp_protocol_request_expects_response((const uint8_t *)"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\"}",
                                                  strlen("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\"}")));
    mcp_json_buffer_destroy(&response);
    (void)puts("mcp protocol: parser, schemas, lifecycle and dispatch passed");
    return EXIT_SUCCESS;
}
