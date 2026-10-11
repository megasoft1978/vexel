#include "svg/animation/animation.h"
#include "shape.h"
#include "shape_render.h"
#include "xml_tokenizer.h"

#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FUZZ_SOURCE_CAP 1024U
#define FUZZ_TOKEN_CAP 256U
#define FUZZ_HIERARCHY_COUNT 6U

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
    static const char *const prefixes[FUZZ_HIERARCHY_COUNT] = {
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<rect x='2' y='2' width='8' height='8' fill='#4f7cff'",
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<g transform='translate(3 4) rotate(7)'><rect x='2' y='2' width='8' height='8' fill='#4f7cff'",
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<g transform='scale(1.1 .9)'><g transform='skewX(4)'><rect x='2' y='2' width='8' height='8' fill='#4f7cff'",
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<defs><g id='asset' transform='translate(2 3)'><rect x='2' y='2' width='8' height='8' fill='#4f7cff'",
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<defs><symbol id='asset' viewBox='0 0 16 16'><rect x='2' y='2' width='8' height='8' fill='#4f7cff'",
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<g transform='translate(4 3)'><svg x='1' y='2' width='24' height='20' viewBox='0 0 16 16'>"
        "<rect x='2' y='2' width='8' height='8' fill='#4f7cff'"
    };
    static const char *const suffixes[FUZZ_HIERARCHY_COUNT] = {
        "</rect></svg>",
        "</rect></g></svg>",
        "</rect></g></g></svg>",
        "</rect></g></defs><g transform='rotate(5)'><use href='#asset'/></g></svg>",
        ("</rect></symbol></defs><g transform='translate(4 2)'>"
         "<use href='#asset' x='2' y='2' width='32' height='32'/></g></svg>"),
        "</rect></svg></g></svg>"
    };
    static const char *const transform_types[] = {
        "translate", "scale", "rotate", "skewX", "skewY"
    };
    static const char *const transform_from[] = {
        "0 0", "1 1", "0", "0", "0"
    };
    static const char *const transform_to[] = {
        "8 4", "1.5 .5", "90", "15", "-15"
    };
    static const char *const additive_modes[] = {"", " additive='sum'",
                                                  " additive='replace'"};
    size_t completed = 0U, parsed = 0U, tracked = 0U;
    size_t decoded = 0U, rendered = 0U;
    size_t hierarchy_counts[FUZZ_HIERARCHY_COUNT] = {0U};
    size_t animation_kinds[4] = {0U};
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
        char source_memory[FUZZ_SOURCE_CAP], animation_markup[512];
        size_t hierarchy = (size_t)(random_next(&state) % FUZZ_HIERARCHY_COUNT);
        size_t animation_kind = (size_t)(random_next(&state) % 4U);
        hierarchy_counts[hierarchy]++;
        animation_kinds[animation_kind]++;
        int animation_length = -1;
        if (animation_kind == 0U) {
            animation_length = snprintf(animation_markup, sizeof(animation_markup),
                "<animate id='item' attributeName='%s' values='0;1' "
                "begin='%s' dur='%s' repeatCount='%s' calcMode='%s' fill='%s'/>",
                attribute, begin, duration, repeat, mode, fill);
        } else if (animation_kind == 1U) {
            size_t transform = (size_t)(random_next(&state) %
                (sizeof(transform_types) / sizeof(transform_types[0])));
            animation_length = snprintf(animation_markup, sizeof(animation_markup),
                "<animateTransform id='item' attributeName='transform' type='%s' "
                "from='%s' to='%s' begin='%s' dur='%s' repeatCount='%s' "
                "fill='%s'%s/>", transform_types[transform],
                transform_from[transform], transform_to[transform], begin,
                duration, repeat, fill,
                additive_modes[random_next(&state) %
                    (sizeof(additive_modes) / sizeof(additive_modes[0]))]);
        } else if (animation_kind == 2U) {
            animation_length = snprintf(animation_markup, sizeof(animation_markup),
                "<animateMotion id='item' path='M0 0 L8 8' begin='%s' dur='%s' "
                "repeatCount='%s' fill='%s'/>", begin, duration, repeat, fill);
        } else {
            animation_length = snprintf(animation_markup, sizeof(animation_markup),
                "<animateTransform id='move' attributeName='transform' "
                "type='translate' from='0 0' to='8 4' begin='%s' dur='%s' "
                "repeatCount='%s' fill='%s'/>"
                "<animateTransform attributeName='transform' type='rotate' "
                "from='0' to='90' begin='%s' dur='%s' repeatCount='%s' "
                "fill='%s'%s/>", begin, duration, repeat, fill, begin,
                duration, repeat, fill,
                additive_modes[random_next(&state) %
                    (sizeof(additive_modes) / sizeof(additive_modes[0]))]);
        }
        if (animation_length < 0 ||
            (size_t)animation_length >= sizeof(animation_markup)) return 3;
        int written = snprintf(source_memory, sizeof(source_memory),
            "%s>%s%s", prefixes[hierarchy], animation_markup,
            suffixes[hierarchy]);
        if (written < 0 || (size_t)written >= sizeof(source_memory)) return 3;
        char source_snapshot[FUZZ_SOURCE_CAP];
        memcpy(source_snapshot, source_memory, (size_t)written);
        base_bytes source = {(const uint8_t *)source_memory, (size_t)written};
        svg_xml_token tokens[FUZZ_TOKEN_CAP] = {{0}};
        svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                               tokens, FUZZ_TOKEN_CAP);
        if (xml.diagnostic.category == SVG_XML_ERROR_NONE) {
            svg_anim_document animation = {0};
            size_t error_offset = 0U;
            if (svg_anim_parse(source, tokens, xml.token_count, &animation,
                               &error_offset)) {
                parsed++;
                if (animation.track_count != 0U) tracked++;
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
                svg_shape_document document = {0};
                svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
                svg_shape_error decode_error = svg_shape_decode(
                    source, &document, &diagnostic);
                if (decode_error == SVG_SHAPE_ERROR_NONE) {
                    decoded++;
                    raster_tile tile = {0};
                    bool rendered_ok = raster_shape_render_at(
                        &document, sample_time, &tile, &diagnostic);
                    if (rendered_ok) rendered++;
                    if ((unsigned)diagnostic.category >
                        (unsigned)SVG_SHAPE_ERROR_UNSUPPORTED_FE_MORPHOLOGY ||
                        diagnostic.offset > source.len) {
                        raster_tile_destroy(&tile);
                        svg_shape_document_destroy(&document);
                        svg_anim_document_destroy(&animation);
                        (void)fprintf(stderr,
                            "invalid render diagnostic in case %" PRIu64
                            " category=%u offset=%zu\n", case_index,
                            (unsigned)diagnostic.category, diagnostic.offset);
                        return 1;
                    }
                    raster_tile_destroy(&tile);
                    svg_shape_document_destroy(&document);
                } else if ((unsigned)decode_error >
                           (unsigned)SVG_SHAPE_ERROR_UNSUPPORTED_FE_MORPHOLOGY ||
                           diagnostic.offset > source.len) {
                    svg_shape_document_destroy(&document);
                    svg_anim_document_destroy(&animation);
                    (void)fprintf(stderr,
                        "invalid decode diagnostic in case %" PRIu64
                        " category=%u offset=%zu\n", case_index,
                        (unsigned)decode_error, diagnostic.offset);
                    return 1;
                }
            }
            svg_anim_document_destroy(&animation);
        }
        if (memcmp(source_memory, source_snapshot, (size_t)written) != 0) {
            (void)fprintf(stderr, "source changed in case %" PRIu64 "\n",
                          case_index);
            return 1;
        }
        completed++;
    }
    (void)printf("animation hierarchy fuzz: %zu cases, parsed=%zu, tracked=%zu, "
                 "decoded=%zu, rendered=%zu, kinds=%zu/%zu/%zu/%zu, "
                 "hierarchies=%zu/%zu/%zu/%zu/%zu/%zu, "
                 "seed-final=%" PRIu64 "\n", completed, parsed, tracked,
                 decoded, rendered, animation_kinds[0], animation_kinds[1],
                 animation_kinds[2], animation_kinds[3], hierarchy_counts[0], hierarchy_counts[1],
                 hierarchy_counts[2], hierarchy_counts[3], hierarchy_counts[4],
                 hierarchy_counts[5], state);
    return completed == (size_t)cases ? 0 : 1;
}
