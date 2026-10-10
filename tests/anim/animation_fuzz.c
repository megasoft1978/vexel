#include "svg/animation/animation.h"
#include "xml_tokenizer.h"

#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FUZZ_SOURCE_CAP 1024U
#define FUZZ_TOKEN_CAP 256U

static uint64_t random_next(uint64_t *state)
{
    uint64_t value = *state;
    value ^= value << 13U;
    value ^= value >> 7U;
    value ^= value << 17U;
    *state = value;
    return value;
}

static bool option_copy(char *out, size_t capacity, const char *const *values,
                        size_t count, uint64_t *state)
{
    size_t index = (size_t)(random_next(state) % count);
    size_t length = strlen(values[index]);
    if (length >= capacity) return false;
    memcpy(out, values[index], length + 1U);
    return true;
}

static bool parse_u64(const char *text, uint64_t maximum, uint64_t *out)
{
    if (text == NULL || text[0] == '\0') return false;
    errno = 0;
    char *end = NULL;
    unsigned long long value = strtoull(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value > maximum) return false;
    *out = (uint64_t)value;
    return true;
}

int main(int argc, char **argv)
{
    uint64_t cases = 200000U, state = 0x6a09e667f3bcc909ULL;
    if ((argc > 1 && !parse_u64(argv[1], 10000000U, &cases)) ||
        (argc > 2 && !parse_u64(argv[2], UINT64_MAX, &state)) || argc > 3 ||
        cases == 0U || state == 0U) return 2;
    static const char *const begins[] = {
        "0s", "0.25s", "2s", "1ms", "indefinite", "click", "onload",
        "item.begin", "item.end+250ms", "item.end-1s", "wallclock(12:00:00)",
        "accessKey(a)", "-1s", "1e999s", "", "999999999999999999999999s"
    };
    static const char *const durations[] = {
        "0.001s", "0.5s", "2s", "3600s", "3601s", "indefinite", "0s",
        "-1s", "nan", "1e999s", "1ms", "", "999999999999999999999999s"
    };
    static const char *const repeats[] = {
        "1", "2", "0.5", "indefinite", "0", "-1", "1e999", "nan", ""
    };
    static const char *const modes[] = {
        "linear", "discrete", "spline", "paced", "unknown", ""
    };
    static const char *const fills[] = {"remove", "freeze", "hold", ""};
    static const char *const attributes[] = {
        "opacity", "x", "width", "fill", "stroke-width", "d", "unknown"
    };
    size_t completed = 0U;
    for (uint64_t case_index = 0U; case_index < cases; ++case_index) {
        char begin[80], duration[80], repeat[80], mode[24], fill[24], attribute[32];
        if (!option_copy(begin, sizeof(begin), begins,
                         sizeof(begins) / sizeof(begins[0]), &state) ||
            !option_copy(duration, sizeof(duration), durations,
                         sizeof(durations) / sizeof(durations[0]), &state) ||
            !option_copy(repeat, sizeof(repeat), repeats,
                         sizeof(repeats) / sizeof(repeats[0]), &state) ||
            !option_copy(mode, sizeof(mode), modes,
                         sizeof(modes) / sizeof(modes[0]), &state) ||
            !option_copy(fill, sizeof(fill), fills,
                         sizeof(fills) / sizeof(fills[0]), &state) ||
            !option_copy(attribute, sizeof(attribute), attributes,
                         sizeof(attributes) / sizeof(attributes[0]), &state))
            return 3;
        char source_memory[FUZZ_SOURCE_CAP];
        int written = snprintf(source_memory, sizeof(source_memory),
            "<svg><rect><animate id='item' attributeName='%s' values='0;1' "
            "begin='%s' dur='%s' repeatCount='%s' calcMode='%s' fill='%s'/>"
            "</rect></svg>", attribute, begin, duration, repeat, mode, fill);
        if (written < 0 || (size_t)written >= sizeof(source_memory)) return 3;
        base_bytes source = {(const uint8_t *)source_memory, (size_t)written};
        svg_xml_token tokens[FUZZ_TOKEN_CAP] = {{0}};
        svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                               tokens, FUZZ_TOKEN_CAP);
        if (xml.diagnostic.category == SVG_XML_ERROR_NONE) {
            svg_anim_document animation = {0};
            size_t error_offset = 0U;
            if (svg_anim_parse(source, tokens, xml.token_count, &animation,
                               &error_offset)) {
                double sample_time = (double)(random_next(&state) % 3600001U) / 1000.0;
                svg_anim_overlay overlay = {0};
                if (!svg_anim_eval(&animation, sample_time, &overlay)) {
                    svg_anim_document_destroy(&animation);
                    (void)fprintf(stderr, "evaluation failed in case %" PRIu64
                        " begin=%s dur=%s repeat=%s mode=%s fill=%s attr=%s\n",
                        case_index, begin, duration, repeat, mode, fill, attribute);
                    return 1;
                }
                svg_anim_overlay_destroy(&overlay);
            }
            svg_anim_document_destroy(&animation);
        }
        completed++;
    }
    (void)printf("animation syntax fuzz: %zu cases, seed-final=%" PRIu64 "\n",
                 completed, state);
    return completed == (size_t)cases ? 0 : 1;
}
