#include "svg/animation/authoring.h"
#include "svg/animation/animation.h"
#include "document_edit.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", \
            __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

/* A rejected or no-op call returns the code and leaves no replacement. */
#define REJECT(call, code, patch) do { \
    svg_anim_authoring_error actual_ = (call); \
    if (actual_ != (code) || (patch).replacement != NULL) { \
        (void)fprintf(stderr, "%s:%d: got %d want %d: %s\n", __FILE__, __LINE__, \
            (int)actual_, (int)(code), #call); \
        return false; \
    } \
} while (0)

#define E_ARG SVG_ANIM_AUTHORING_INVALID_ARGUMENT
#define E_UNS SVG_ANIM_AUTHORING_UNSUPPORTED
#define E_NOP SVG_ANIM_AUTHORING_NO_OP
#define BYTES(text) ((base_bytes){(const uint8_t *)(text), sizeof(text) - 1U})

static bool parse_animation(base_bytes source, svg_anim_document *out)
{
    size_t capacity = source.len / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) return false;
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) return false;
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
        tokens, capacity);
    bool parsed = xml.diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_anim_parse(source, tokens, xml.token_count, out, NULL);
    free(tokens);
    return parsed;
}

static bool contains(base_bytes source, const char *text)
{
    size_t length = strlen(text);
    for (size_t index = 0U; index + length <= source.len; ++index)
        if (memcmp(source.ptr + index, text, length) == 0) return true;
    return false;
}

/* Applies the patch through document-edit; checks byte-exact undo and redo. */
static bool apply_checked(base_bytes source, const svg_anim_authoring_patch *patch,
                          uint8_t **out_after, size_t *out_length)
{
    size_t capacity = source.len + patch->replacement_length + 32U;
    uint8_t *expected = malloc(capacity), *edited = malloc(capacity);
    uint8_t *scratch = malloc(capacity);
    CHECK(expected != NULL && edited != NULL && scratch != NULL);
    memcpy(expected, source.ptr, patch->offset);
    memcpy(expected + patch->offset, patch->replacement, patch->replacement_length);
    size_t suffix = source.len - patch->offset - patch->remove_length;
    memcpy(expected + patch->offset + patch->replacement_length,
        source.ptr + patch->offset + patch->remove_length, suffix);
    size_t expected_length = patch->offset + patch->replacement_length + suffix;
    document_edit_result created = document_edit_create(source,
        document_edit_default_limits());
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    CHECK(document_edit_replace_source_span(created.edit,
        (document_source_span){patch->offset, patch->remove_length},
        (base_bytes){patch->replacement, patch->replacement_length}) == DOCUMENT_EDIT_OK);
    size_t edited_length = 0U, scratch_length = 0U;
    CHECK(document_edit_materialize(created.edit, edited, capacity, &edited_length));
    CHECK(edited_length == expected_length && memcmp(edited, expected, edited_length) == 0);
    CHECK(document_edit_undo(created.edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_materialize(created.edit, scratch, capacity, &scratch_length));
    CHECK(scratch_length == source.len && memcmp(scratch, source.ptr, source.len) == 0);
    CHECK(document_edit_redo(created.edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_materialize(created.edit, scratch, capacity, &scratch_length));
    CHECK(scratch_length == edited_length && memcmp(scratch, edited, edited_length) == 0);
    document_edit_destroy(created.edit);
    free(scratch);
    free(expected);
    *out_after = edited;
    *out_length = edited_length;
    return true;
}

static bool test_null_patch_and_bad_track_index(void)
{
    static const char text[] =
        "<svg><rect><animate attributeName='x' values='0;5;10' keyTimes='0;0.5;1' "
        "dur='2s'/></rect></svg>";
    base_bytes source = BYTES(text);
    svg_anim_document animation = {0};
    CHECK(parse_animation(source, &animation));
    double spline[4] = {0.4, 0.0, 0.6, 1.0};
    uint8_t *fragment = NULL;
    size_t length = 0U;
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 0U, BYTES("1"), NULL) == E_ARG);
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 0.5, BYTES("1"), NULL) == E_ARG);
    CHECK(svg_anim_authoring_move_key(&animation, 0U, 1U, 0.5, NULL) == E_ARG);
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 1U, NULL) == E_ARG);
    CHECK(svg_anim_authoring_set_calc_mode(&animation, 0U, SVG_ANIM_CALC_DISCRETE, NULL) == E_ARG);
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, 0.0, 3.0, NULL) == E_ARG);
    CHECK(svg_anim_authoring_set_spline(&animation, 0U, 0U, spline, NULL) == E_ARG);
    CHECK(svg_anim_authoring_create_track(source, 6U, SVG_ANIM_ATTR_X,
        SVG_ANIM_KIND_ANIMATE, SVG_ANIM_TRANSFORM_TRANSLATE, BYTES("0"), BYTES("1"),
        1.0, 2.0, NULL, &length) == E_ARG);
    CHECK(svg_anim_authoring_create_track(source, 6U, SVG_ANIM_ATTR_X,
        SVG_ANIM_KIND_ANIMATE, SVG_ANIM_TRANSFORM_TRANSLATE, BYTES("0"), BYTES("1"),
        1.0, 2.0, &fragment, NULL) == E_ARG);

    svg_anim_authoring_patch patch = {0};
    const svg_anim_document *documents[2] = {&animation, NULL};
    size_t indexes[2] = {animation.track_count, 0U};
    for (size_t i = 0U; i < 2U; ++i) {
        REJECT(svg_anim_authoring_set_key_value(documents[i], indexes[i], 0U,
            BYTES("1"), &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_add_key(documents[i], indexes[i], 0.5,
            BYTES("1"), &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_move_key(documents[i], indexes[i], 1U, 0.6,
            &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_delete_key(documents[i], indexes[i], 1U,
            &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_set_calc_mode(documents[i], indexes[i],
            SVG_ANIM_CALC_DISCRETE, &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_set_timing(documents[i], indexes[i], 0.0, 3.0,
            &patch), E_UNS, patch);
        REJECT(svg_anim_authoring_set_spline(documents[i], indexes[i], 0U, spline,
            &patch), E_UNS, patch);
    }
    CHECK(!svg_anim_authoring_track_editable(&animation, animation.track_count));
    svg_anim_document_destroy(&animation);
    return true;
}

/* Runs every patch-producing op on a document with a damaged track. */
static bool expect_all_unsupported(const svg_anim_document *document)
{
    svg_anim_authoring_patch patch = {0};
    REJECT(svg_anim_authoring_set_key_value(document, 0U, 0U, BYTES("7"), &patch),
        E_UNS, patch);
    REJECT(svg_anim_authoring_add_key(document, 0U, 0.6, BYTES("7"), &patch),
        E_UNS, patch);
    REJECT(svg_anim_authoring_move_key(document, 0U, 1U, 1.2, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_delete_key(document, 0U, 1U, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_set_calc_mode(document, 0U, SVG_ANIM_CALC_DISCRETE,
        &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_set_timing(document, 0U, 0.5, 3.0, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_set_spline(document, 0U, 0U,
        (double[4]){0.4, 0.0, 0.6, 1.0}, &patch), E_UNS, patch);
    return true;
}

static bool test_corrupt_spans_and_oversized_tags(void)
{
    static const char text[] =
        "<svg><rect><animate attributeName='x' values='0;5;10' keyTimes='0;0.5;1' "
        "dur='2s'/></rect></svg>";
    base_bytes source = BYTES(text);
    svg_anim_document animation = {0};
    CHECK(parse_animation(source, &animation));
    svg_anim_document copy = animation;
    svg_anim_track *tracks = malloc(animation.track_count * sizeof(*tracks));
    CHECK(tracks != NULL);
    memcpy(tracks, animation.tracks, animation.track_count * sizeof(*tracks));
    copy.tracks = tracks;
    size_t offset = tracks[0].element_offset, length = tracks[0].element_length;
    tracks[0].element_offset = source.len;
    CHECK(expect_all_unsupported(&copy));
    tracks[0].element_offset = offset;
    tracks[0].element_length = source.len + 1U;
    CHECK(expect_all_unsupported(&copy));
    tracks[0].element_offset = offset + 1U;
    tracks[0].element_length = length - 1U;
    CHECK(expect_all_unsupported(&copy));
    CHECK(!svg_anim_authoring_track_removable(&copy, 0U));
    free(tracks);
    svg_anim_document_destroy(&animation);

    const size_t big = 64U * 1024U + 16U;
    char *large = malloc(big + 512U);
    CHECK(large != NULL);
    int used = snprintf(large, 512U, "<svg><rect><animate attributeName='x' "
        "values='0;5;10' keyTimes='0;0.5;1' dur='2s' data-pad='");
    CHECK(used > 0);
    memset(large + used, 'a', big);
    size_t total = (size_t)used + big;
    static const char tail[] = "'/></rect></svg>";
    memcpy(large + total, tail, sizeof(tail) - 1U);
    total += sizeof(tail) - 1U;
    CHECK(parse_animation((base_bytes){(const uint8_t *)large, total}, &animation));
    CHECK(expect_all_unsupported(&animation));
    svg_anim_document_destroy(&animation);
    free(large);

    char many[4096];
    used = snprintf(many, sizeof(many), "<svg><rect><animate attributeName='x' "
        "values='0;5;10' keyTimes='0;0.5;1' dur='2s'");
    CHECK(used > 0);
    for (size_t index = 0U; index < 130U; ++index) {
        int added = snprintf(many + used, sizeof(many) - (size_t)used,
            " d%zu='1'", index);
        CHECK(added > 0);
        used += added;
    }
    int tail_used = snprintf(many + used, sizeof(many) - (size_t)used, "/></rect></svg>");
    CHECK(tail_used > 0);
    CHECK(parse_animation((base_bytes){(const uint8_t *)many,
        (size_t)(used + tail_used)}, &animation));
    CHECK(expect_all_unsupported(&animation));
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_key_operation_bounds(void)
{
    static const char text[] =
        "<svg><rect><animate attributeName='x' values='0;1;2;3' "
        "keyTimes='0;0.25;0.5;1' dur='4s'/></rect></svg>";
    base_bytes source = BYTES(text);
    svg_anim_document animation = {0};
    svg_anim_authoring_patch patch = {0};
    CHECK(parse_animation(source, &animation));
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 4U, BYTES("1"), &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 1U, BYTES(""), &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 1U, BYTES("1;2"), &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 1U,
        (base_bytes){(const uint8_t *)"1\0" "2", 3U}, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 1U, BYTES("abc"), &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_key_value(&animation, 0U, 1U, (base_bytes){NULL, 1U}, &patch), E_ARG, patch);

    REJECT(svg_anim_authoring_add_key(&animation, 0U, 0.0, BYTES("9"), &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_add_key(&animation, 0U, 4.0, BYTES("9"), &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_add_key(&animation, 0U, NAN, BYTES("9"), &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_add_key(&animation, 0U, INFINITY, BYTES("9"), &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_add_key(&animation, 0U, 2.0, BYTES("9"), &patch), E_ARG, patch);
    uint8_t *huge = malloc(64U * 1024U + 1U);
    CHECK(huge != NULL);
    memset(huge, '1', 64U * 1024U + 1U);
    REJECT(svg_anim_authoring_add_key(&animation, 0U, 1.5,
        (base_bytes){huge, 64U * 1024U + 1U}, &patch), E_ARG, patch);
    free(huge);

    REJECT(svg_anim_authoring_move_key(&animation, 0U, 0U, 1.5, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 3U, 3.5, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 1U, 2.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 1U, 3.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 2U, 1.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 2U, 0.5, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 1U, 1.0, &patch), E_NOP, patch);
    REJECT(svg_anim_authoring_delete_key(&animation, 0U, 4U, &patch), E_UNS, patch);
    svg_anim_document_destroy(&animation);

    static const char single[] =
        "<svg><rect><animate attributeName='x' values='5' dur='2s'/></rect></svg>";
    CHECK(parse_animation(BYTES(single), &animation));
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 0U, 1.0, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_move_key(&animation, 0U, 1U, 1.0, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_delete_key(&animation, 0U, 0U, &patch), E_UNS, patch);
    svg_anim_document_destroy(&animation);

    static const char renormalize[] =
        "<svg><rect><animate attributeName='x' values='0;1;2' "
        "keyTimes='0;0.25;1' dur='4s'/></rect></svg>";
    CHECK(parse_animation(BYTES(renormalize), &animation));
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 0U, &patch) == SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_checked(BYTES(renormalize), &patch, &after, &after_length));
    CHECK(contains((base_bytes){after, after_length}, "values='1;2'") &&
        contains((base_bytes){after, after_length}, "keyTimes='0;1'"));
    free(after);
    svg_anim_authoring_patch_destroy(&patch);
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_timing_spline_and_mode_bounds(void)
{
    static const char text[] =
        "<svg><rect><animate attributeName='x' values='0;5' dur='2s'/></rect></svg>";
    base_bytes source = BYTES(text);
    svg_anim_document animation = {0};
    svg_anim_authoring_patch patch = {0};
    CHECK(parse_animation(source, &animation));
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, NAN, 2.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, INFINITY, 2.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, 0.0, NAN, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, 0.0, INFINITY, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, 0.0, 0.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, 0.0, -1.0, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_timing(&animation, 0U, 1.0, 3599.5, &patch), E_ARG, patch);
    CHECK(svg_anim_authoring_set_timing(&animation, 0U, 1.0, 3599.0, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    uint8_t *after = NULL;
    size_t after_length = 0U;
    CHECK(apply_checked(source, &patch, &after, &after_length));
    svg_anim_document edited = {0};
    CHECK(parse_animation((base_bytes){after, after_length}, &edited));
    CHECK(edited.tracks[0].begin_offset == 1.0 &&
        edited.tracks[0].duration_seconds == 3599.0);
    svg_anim_document_destroy(&edited);
    free(after);
    svg_anim_authoring_patch_destroy(&patch);

    REJECT(svg_anim_authoring_set_spline(&animation, 0U, 0U,
        (double[4]){NAN, 0.0, 0.5, 1.0}, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_spline(&animation, 0U, 0U,
        (double[4]){-0.1, 0.0, 0.5, 1.0}, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_spline(&animation, 0U, 0U, NULL, &patch), E_ARG, patch);
    REJECT(svg_anim_authoring_set_spline(&animation, 0U, 1U,
        (double[4]){0.4, 0.0, 0.6, 1.0}, &patch), E_UNS, patch);
    REJECT(svg_anim_authoring_set_calc_mode(&animation, 0U, SVG_ANIM_CALC_PACED,
        &patch), E_UNS, patch);
    svg_anim_document_destroy(&animation);
    return true;
}

static bool test_create_track_bounds(void)
{
    static const char text[] = "<svg><rect id='t'/></svg>";
    base_bytes source = BYTES(text);
    uint8_t *fragment = NULL;
    size_t length = 0U;
#define CREATE(attribute, kind, type, init, first, time, dur, offset, code) do { \
        svg_anim_authoring_error e_ = svg_anim_authoring_create_track(source, \
            offset, attribute, kind, type, BYTES(init), BYTES(first), time, dur, \
            &fragment, &length); \
        CHECK(e_ == (code) && fragment == NULL && length == 0U); \
    } while (0)
    const svg_anim_transform_type tr = SVG_ANIM_TRANSFORM_TRANSLATE;
    CREATE(SVG_ANIM_ATTR_D, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 1.0, 2.0, 6U, E_UNS);
    CREATE(SVG_ANIM_ATTR_MOTION, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 1.0, 2.0, 6U, E_UNS);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_TRANSFORM, tr, "0", "1", 1.0, 2.0, 6U, E_UNS);
    CREATE(SVG_ANIM_ATTR_TRANSFORM, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 1.0, 2.0, 6U, E_UNS);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_SET, tr, "0", "1", 1.0, 2.0, 6U, E_UNS);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 1.0, 2.0, source.len, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 0.0, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", NAN, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 3.0, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1", 1.0, 3601.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "abc", "1", 1.0, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "abc", 1.0, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "", "1", 1.0, 2.0, 6U, E_ARG);
    CREATE(SVG_ANIM_ATTR_X, SVG_ANIM_KIND_ANIMATE, tr, "0", "1;2", 1.0, 2.0, 6U, E_ARG);
#undef CREATE
    CHECK(svg_anim_authoring_create_track(source, 6U, SVG_ANIM_ATTR_X,
        SVG_ANIM_KIND_ANIMATE, tr, BYTES("0"), BYTES("9"), 2.0, 2.0, &fragment,
        &length) == SVG_ANIM_AUTHORING_OK);
    base_bytes made = {fragment, length};
    CHECK(contains(made, "values=\"0;9\"") && contains(made, "keyTimes=\"0;1\"") &&
        !contains(made, ";1;1") && !contains(made, "9;9"));
    free(fragment);
    return true;
}

static const char spline_text[] =
    "<svg><rect><animate attributeName='x' values='0;1;2;3' dur='3s' "
    "calcMode='spline' keySplines='0.10 0.20 0.30 0.40;  0.22 0.32 0.42 0.52 ;"
    "0.40 0.50 0.60 0.70'/></rect></svg>";
#define T0 "0.10 0.20 0.30 0.40"
#define T1 "  0.22 0.32 0.42 0.52 "
#define T2 "0.40 0.50 0.60 0.70"

/* Runs one op's patch through undo/redo, reparses, and checks the tuples. */
static bool check_spline_result(base_bytes source, svg_anim_authoring_patch *patch,
    size_t keys, const char *tuples)
{
    uint8_t *after = NULL;
    size_t length = 0U;
    CHECK(apply_checked(source, patch, &after, &length));
    svg_anim_authoring_patch_destroy(patch);
    svg_anim_document parsed = {0};
    base_bytes edited = {after, length};
    CHECK(parse_animation(edited, &parsed));
    CHECK(parsed.track_count == 1U && parsed.tracks[0].calc_mode == SVG_ANIM_CALC_SPLINE);
    CHECK(parsed.tracks[0].key_count == keys && parsed.tracks[0].spline_count == keys - 1U);
    char expected[256];
    CHECK(snprintf(expected, sizeof(expected), "keySplines='%s'", tuples) > 0);
    CHECK(contains(edited, expected));
    svg_anim_document_destroy(&parsed);
    free(after);
    return true;
}

static bool test_spline_track_key_edits(void)
{
    base_bytes source = BYTES(spline_text);
    svg_anim_document animation = {0};
    svg_anim_authoring_patch patch = {0};
    CHECK(parse_animation(source, &animation));
    CHECK(svg_anim_authoring_track_editable(&animation, 0U));
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 1.5, BYTES("7"), &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 5U, T0 ";" T1 ";" T1 ";" T2));
    CHECK(svg_anim_authoring_add_key(&animation, 0U, 0.5, BYTES("7"), &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 5U, T0 ";" T0 ";" T1 ";" T2));
    CHECK(svg_anim_authoring_set_key_value(&animation, 0U, 1U, BYTES("7"), &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 4U, T0 ";" T1 ";" T2));
    CHECK(svg_anim_authoring_move_key(&animation, 0U, 1U, 1.2, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 4U, T0 ";" T1 ";" T2));
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 0U, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 3U, T1 ";" T2));
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 1U, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 3U, T0 ";" T2));
    CHECK(svg_anim_authoring_delete_key(&animation, 0U, 3U, &patch) ==
        SVG_ANIM_AUTHORING_OK);
    CHECK(check_spline_result(source, &patch, 3U, T0 ";" T1));
    svg_anim_document_destroy(&animation);

    static const char two_text[] =
        "<svg><rect><animate attributeName='x' values='0;5' dur='1s' "
        "calcMode='spline' keySplines='0.1 0.2 0.3 0.4'/></rect></svg>";
    base_bytes two = BYTES(two_text);
    CHECK(parse_animation(two, &animation));
    for (size_t key = 0U; key < 2U; ++key) {
        CHECK(svg_anim_authoring_delete_key(&animation, 0U, key, &patch) ==
            SVG_ANIM_AUTHORING_OK);
        uint8_t *after = NULL;
        size_t length = 0U;
        CHECK(apply_checked(two, &patch, &after, &length));
        svg_anim_authoring_patch_destroy(&patch);
        svg_anim_document parsed = {0};
        CHECK(parse_animation((base_bytes){after, length}, &parsed));
        CHECK(parsed.tracks[0].key_count == 1U &&
            parsed.tracks[0].calc_mode == SVG_ANIM_CALC_LINEAR &&
            parsed.tracks[0].spline_count == 0U);
        CHECK(!contains((base_bytes){after, length}, "keySplines") &&
            contains((base_bytes){after, length}, "calcMode='linear'") &&
            contains((base_bytes){after, length}, key == 0U ? "values='5'" : "values='0'"));
        svg_anim_document_destroy(&parsed);
        free(after);
    }
    svg_anim_document_destroy(&animation);
    return true;
}

/* Whitespace padding is untrimmed raw text, so a duplicated tuple can push
 * keySplines past 64 KiB even though the parsed tuple is tiny. */
static bool test_spline_output_limit(void)
{
    enum { PAD = 40000 };
    static const char head[] =
        "<svg><rect><animate attributeName='x' values='0;1;2' dur='2s' "
        "calcMode='spline' keySplines='";
    static const char tail[] = "0 0 1 1;0 0 1 1'/></rect></svg>";
    size_t length = sizeof(head) - 1U + PAD + sizeof(tail) - 1U;
    uint8_t *text = malloc(length);
    CHECK(text != NULL);
    memcpy(text, head, sizeof(head) - 1U);
    memset(text + sizeof(head) - 1U, ' ', PAD);
    memcpy(text + sizeof(head) - 1U + PAD, tail, sizeof(tail) - 1U);
    svg_anim_document animation = {0};
    svg_anim_authoring_patch patch = {0};
    bool parsed = parse_animation((base_bytes){text, length}, &animation);
    free(text);
    CHECK(parsed && svg_anim_authoring_track_editable(&animation, 0U));
    REJECT(svg_anim_authoring_add_key(&animation, 0U, 0.5, BYTES("7"), &patch),
        SVG_ANIM_AUTHORING_LIMIT, patch);
    svg_anim_document_destroy(&animation);
    return true;
}

int main(void)
{
    if (!test_null_patch_and_bad_track_index() ||
        !test_corrupt_spans_and_oversized_tags() ||
        !test_key_operation_bounds() ||
        !test_timing_spline_and_mode_bounds() ||
        !test_create_track_bounds() ||
        !test_spline_track_key_edits() ||
        !test_spline_output_limit()) return EXIT_FAILURE;
    (void)puts("animation authoring edge: all checks passed");
    return EXIT_SUCCESS;
}
