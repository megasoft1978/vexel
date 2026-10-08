#include "xml_tokenizer.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define MAX_INPUT 2048U
#define MAX_TOKENS 2048U

static double now_seconds(void)
{
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) return 0.0;
    return (double)value.tv_sec + (double)value.tv_nsec / 1000000000.0;
}

static uint32_t next_random(uint32_t *state)
{
    *state ^= *state << 13U;
    *state ^= *state >> 17U;
    *state ^= *state << 5U;
    return *state;
}

static uint32_t iteration_seed(uint32_t seed, uint64_t iteration)
{
    uint32_t state = seed ^ (uint32_t)iteration ^ (uint32_t)(iteration >> 32U) ^ 0x9e3779b9U;
    return state == 0 ? 0x6d2b79f5U : state;
}

static int invariant(base_bytes input, const svg_xml_result *result, const svg_xml_token *tokens)
{
    if (result->diagnostic.offset > input.len) return 0;
    if (result->diagnostic.category != SVG_XML_ERROR_NONE) return result->token_count == 0;
    for (size_t i = 0; i < result->token_count; ++i) {
        const svg_xml_span spans[] = {tokens[i].source, tokens[i].name, tokens[i].value,
                                      tokens[i].prefix, tokens[i].local_name};
        for (size_t j = 0; j < sizeof(spans) / sizeof(spans[0]); ++j)
            if (spans[j].offset > input.len || spans[j].length > input.len - spans[j].offset) return 0;
    }
    return 1;
}

static size_t make_input(uint32_t seed, uint64_t iteration, uint8_t *bytes)
{
    static const char *const motifs[] = {"<a/>", "<svg xmlns='urn:x'><g></svg>", "<!--x-->",
        "<![CDATA[x]]>", "<?p x?>", "<a x='&amp;'/>", "<!DOCTYPE a><a/>", "<a>]]></a>",
        "<a x=\"x\" x='y'/>", "</a>"};
    uint32_t state = iteration_seed(seed, iteration);
    size_t length = (size_t)(next_random(&state) % MAX_INPUT);
    for (size_t i = 0; i < length; ++i) bytes[i] = (uint8_t)next_random(&state);
    if (iteration % 3U == 0) {
        const char *motif = motifs[next_random(&state) % (sizeof(motifs) / sizeof(motifs[0]))];
        length = strlen(motif);
        memcpy(bytes, motif, length);
    }
    return length;
}

static int run_input(uint8_t *bytes, size_t length, uint64_t iteration)
{
    svg_xml_token tokens[MAX_TOKENS];
    base_bytes input = {bytes, length};
    svg_xml_result result = svg_xml_tokenize(input, svg_xml_default_options(), tokens, MAX_TOKENS);
    if (!invariant(input, &result, tokens)) {
        fprintf(stderr, "span/offset invariant failed at iteration %llu\n", (unsigned long long)iteration);
        return 4;
    }
    return 0;
}

static int parse_u64(const char *text, uint64_t *out)
{
    char *end = NULL;
    unsigned long long value = strtoull(text, &end, 10);
    if (end == text || *end != '\0') return 0;
    *out = (uint64_t)value;
    return 1;
}

static int parse_seed(const char *text, uint32_t *out)
{
    uint64_t value;
    if (!parse_u64(text, &value) || value > UINT32_MAX) return 0;
    *out = (uint32_t)value;
    return 1;
}

static int replay(uint32_t seed, uint64_t iteration)
{
    uint8_t bytes[MAX_INPUT];
    size_t length = make_input(seed, iteration, bytes);
    printf("{\"seed\":%u,\"iteration\":%llu,\"input_hex\":\"", seed,
           (unsigned long long)iteration);
    for (size_t i = 0; i < length; ++i) printf("%02x", bytes[i]);
    puts("\"}");
    if (fflush(stdout) != 0) return 6;
    return run_input(bytes, length, iteration);
}

static int one_input(const char *hex, uint64_t iteration)
{
    size_t digits = strlen(hex);
    if ((digits & 1U) != 0 || digits / 2U > MAX_INPUT) return 2;
    uint8_t bytes[MAX_INPUT];
    size_t length = digits / 2U;
    for (size_t i = 0; i < length; ++i) {
        unsigned value;
        if (sscanf(hex + 2U * i, "%2x", &value) != 1) return 2;
        bytes[i] = (uint8_t)value;
    }
    return run_input(bytes, length, iteration);
}

int main(int argc, char **argv)
{
    if (argc == 4 && strcmp(argv[1], "--replay") == 0) {
        uint32_t seed;
        uint64_t iteration;
        if (!parse_seed(argv[2], &seed) || !parse_u64(argv[3], &iteration)) return 2;
        return replay(seed, iteration);
    }
    if (argc == 4 && strcmp(argv[1], "--one-input-hex") == 0) {
        uint64_t iteration;
        if (!parse_u64(argv[3], &iteration)) return 2;
        return one_input(argv[2], iteration);
    }
    if (argc != 4 || strcmp(argv[1], "--campaign") != 0) return 2;
    uint32_t seed;
    uint64_t duration;
    if (!parse_seed(argv[2], &seed) || !parse_u64(argv[3], &duration) || duration == 0 || duration > 600) return 2;
    uint64_t iteration = 0;
    double start = now_seconds(), deadline = start + (double)duration;
    uint8_t bytes[MAX_INPUT];
    while (now_seconds() < deadline) {
        size_t length = make_input(seed, iteration, bytes);
        int status = run_input(bytes, length, iteration);
        if (status != 0) return status;
        ++iteration;
        if (iteration % 128U == 0) {
            printf("PROGRESS %llu\n", (unsigned long long)iteration);
            fflush(stdout);
        }
    }
    double elapsed = now_seconds() - start;
    printf("{\"seed\":%u,\"iterations\":%llu,\"elapsed_seconds\":%.6f,\"sanitizer_status\":\"clean\"}\n",
           seed, (unsigned long long)iteration, elapsed);
    return elapsed + 0.005 < (double)duration ? 5 : 0;
}
