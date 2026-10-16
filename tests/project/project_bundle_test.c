#include "project_bundle.h"
#include "project_bundle_asset_overlay.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool equal(base_bytes left, base_bytes right)
{
    return left.len == right.len && (left.len == 0U ||
        memcmp(left.ptr, right.ptr, left.len) == 0);
}

static uint32_t test_crc32_update(uint32_t crc, base_bytes bytes_value)
{
    for (size_t index = 0U; index < bytes_value.len; index++) {
        crc ^= bytes_value.ptr[index];
        for (unsigned bit = 0U; bit < 8U; bit++) {
            uint32_t mask = (uint32_t)-(int32_t)(crc & 1U);
            crc = (crc >> 1U) ^ (0xedb88320U & mask);
        }
    }
    return crc;
}

static uint32_t test_asset_crc32(base_bytes name, base_bytes media, base_bytes data)
{
    uint32_t crc = test_crc32_update(UINT32_MAX, name);
    crc = test_crc32_update(crc, media);
    return ~test_crc32_update(crc, data);
}

static void put_u16(uint8_t *bytes_value, uint16_t value)
{
    bytes_value[0] = (uint8_t)value;
    bytes_value[1] = (uint8_t)(value >> 8U);
}

static void put_u32(uint8_t *bytes_value, uint32_t value)
{
    for (size_t index = 0U; index < 4U; index++)
        bytes_value[index] = (uint8_t)(value >> (index * 8U));
}

static void put_u64(uint8_t *bytes_value, uint64_t value)
{
    for (size_t index = 0U; index < 8U; index++)
        bytes_value[index] = (uint8_t)(value >> (index * 8U));
}

static bool test_round_trip(void)
{
    static const uint8_t asset_data[] = {0U, 1U, 0xffU, 0x20U};
    project_bundle_asset assets[] = {
        {bytes("character.png"), bytes("image/png"), {asset_data, sizeof(asset_data)}},
        {bytes("audio-1.wav"), bytes("audio/wav"), {NULL, 0U}}
    };
    project_bundle_input input = {
        bytes("{\"title\":\"Sketch\",\"version\":1}"),
        bytes("<svg xmlns='http://www.w3.org/2000/svg'><!--keep--></svg>"),
        bytes("{\"fps\":24,\"tracks\":[{\"name\":\"walk\"}]}"),
        assets, sizeof(assets) / sizeof(assets[0])
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK && measured.encoded_size > 56U);
    uint8_t encoded[4096] = {0};
    project_bundle_result written = project_bundle_encode(&input,
        (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(written.error == PROJECT_BUNDLE_OK && written.encoded_size == measured.encoded_size);
    project_bundle_view view = {0};
    project_bundle_result parsed = project_bundle_decode((base_bytes){encoded, written.encoded_size}, &view);
    CHECK(parsed.error == PROJECT_BUNDLE_OK && view.version == PROJECT_BUNDLE_VERSION);
    CHECK(equal(input.manifest_json, view.manifest_json));
    CHECK(equal(input.scene_svg, view.scene_svg));
    CHECK(equal(input.timeline_json, view.timeline_json));
    CHECK(view.asset_count == 2U);
    for (size_t index = 0U; index < view.asset_count; index++) {
        project_bundle_asset actual;
        CHECK(project_bundle_asset_at(&view, index, &actual));
        CHECK(equal(assets[index].name, actual.name));
        CHECK(equal(assets[index].media_type, actual.media_type));
        CHECK(equal(assets[index].data, actual.data));
    }
    project_bundle_asset decoded_assets[2];
    for (size_t index = 0U; index < view.asset_count; index++)
        CHECK(project_bundle_asset_at(&view, index, &decoded_assets[index]));
    project_bundle_input decoded_input = {view.manifest_json, view.scene_svg,
        view.timeline_json, decoded_assets, view.asset_count};
    uint8_t second[4096] = {0};
    written = project_bundle_encode(&decoded_input, (base_mut_bytes){second, sizeof(second)});
    CHECK(written.error == PROJECT_BUNDLE_OK && written.encoded_size == parsed.encoded_size);
    CHECK(memcmp(encoded, second, parsed.encoded_size) == 0);
    return true;
}

static bool test_crc_vector_and_empty_timeline(void)
{
    project_bundle_input input = {bytes("{}"), bytes("<svg/>"), {NULL, 0U}, NULL, 0U};
    uint8_t encoded[128] = {0};
    project_bundle_result write = project_bundle_encode(&input,
        (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(write.error == PROJECT_BUNDLE_OK);
    /* CRC-32/IEEE("123456789") is the standard check value CBF43926. */
    project_bundle_input crc_input = {bytes("123456789"), bytes("<svg/>"), {NULL, 0U}, NULL, 0U};
    write = project_bundle_encode(&crc_input, (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(write.error == PROJECT_BUNDLE_OK);
    CHECK(encoded[40] == 0x26U && encoded[41] == 0x39U &&
          encoded[42] == 0xf4U && encoded[43] == 0xcbU);
    project_bundle_view view;
    CHECK(project_bundle_decode((base_bytes){encoded, write.encoded_size}, &view).error == PROJECT_BUNDLE_OK);
    return true;
}

static bool test_insufficient_and_invalid_inputs(void)
{
    project_bundle_input input = {bytes("{}"), bytes("<svg/>"), bytes("{}"), NULL, 0U};
    uint8_t output[128];
    memset(output, 0xa5, sizeof(output));
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK);
    project_bundle_result small = project_bundle_encode(&input,
        (base_mut_bytes){output, measured.encoded_size - 1U});
    CHECK(small.error == PROJECT_BUNDLE_BUFFER_TOO_SMALL && small.encoded_size == measured.encoded_size);
    for (size_t index = 0U; index < sizeof(output); index++) CHECK(output[index] == 0xa5U);
    project_bundle_asset invalid_name = {bytes("../escape"), bytes("image/png"), bytes("x")};
    input.assets = &invalid_name; input.asset_count = 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    project_bundle_asset bad_mime = {bytes("safe.png"), bytes("image//png"), bytes("x")};
    input.assets = &bad_mime;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    project_bundle_asset duplicate[2] = {
        {bytes("same"), bytes("image/png"), bytes("a")},
        {bytes("same"), bytes("image/jpeg"), bytes("b")}
    };
    input.assets = duplicate; input.asset_count = 2U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    input.assets = NULL; input.asset_count = 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    input.asset_count = 0U;
    input.manifest_json.len = PROJECT_BUNDLE_MAX_MANIFEST_BYTES + 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    input.manifest_json = bytes("{}");
    input.scene_svg.len = PROJECT_BUNDLE_MAX_SCENE_BYTES + 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    input.scene_svg = bytes("<svg/>");
    input.timeline_json.len = PROJECT_BUNDLE_MAX_TIMELINE_BYTES + 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    input.timeline_json = bytes("{}");
    project_bundle_asset over_count_asset = {bytes("one"), bytes("image/png"), bytes("x")};
    input.assets = &over_count_asset;
    input.asset_count = PROJECT_BUNDLE_MAX_ASSET_COUNT + 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    project_bundle_asset over_size_asset = {bytes("large"), bytes("image/png"),
        {(const uint8_t *)"x", PROJECT_BUNDLE_MAX_ASSET_BYTES + 1U}};
    input.assets = &over_size_asset;
    input.asset_count = 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    project_bundle_asset over_total_assets[3] = {
        {bytes("large-a"), bytes("image/png"), {(const uint8_t *)"x", 200U * 1024U * 1024U}},
        {bytes("large-b"), bytes("image/png"), {(const uint8_t *)"x", 200U * 1024U * 1024U}},
        {bytes("large-c"), bytes("image/png"), {(const uint8_t *)"x", 200U * 1024U * 1024U}}
    };
    input.assets = over_total_assets;
    input.asset_count = 3U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_LIMIT);
    input.assets = NULL;
    input.asset_count = 0U;
    uint8_t overlapping[128] = {0};
    memcpy(overlapping + 4U, "<svg/>", 6U);
    input.scene_svg = (base_bytes){overlapping + 4U, 6U};
    CHECK(project_bundle_encode(&input, (base_mut_bytes){overlapping, sizeof(overlapping)}).error ==
          PROJECT_BUNDLE_INVALID_ARGUMENT);
    union {
        project_bundle_input descriptor;
        uint8_t storage[sizeof(project_bundle_input) + 128U];
    } descriptor_storage = {0};
    descriptor_storage.descriptor = (project_bundle_input){bytes("{}"), bytes("<svg/>"),
        bytes("{}"), NULL, 0U};
    CHECK(project_bundle_encode(&descriptor_storage.descriptor,
        (base_mut_bytes){descriptor_storage.storage, sizeof(descriptor_storage.storage)}).error ==
        PROJECT_BUNDLE_INVALID_ARGUMENT);
    union {
        project_bundle_asset asset;
        uint8_t storage[sizeof(project_bundle_asset) + 128U];
    } asset_storage = {0};
    asset_storage.asset = (project_bundle_asset){bytes("same"), bytes("image/png"), bytes("x")};
    input = (project_bundle_input){bytes("{}"), bytes("<svg/>"), bytes("{}"),
        &asset_storage.asset, 1U};
    CHECK(project_bundle_encode(&input, (base_mut_bytes){asset_storage.storage,
        sizeof(asset_storage.storage)}).error == PROJECT_BUNDLE_INVALID_ARGUMENT);
    return true;
}

static bool test_exact_limits(void)
{
    static const uint8_t byte = 0U;
    project_bundle_input input = {{&byte, PROJECT_BUNDLE_MAX_MANIFEST_BYTES},
        bytes("<svg/>"), bytes("{}"), NULL, 0U};
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);
    input.manifest_json = bytes("{}");
    input.scene_svg = (base_bytes){&byte, PROJECT_BUNDLE_MAX_SCENE_BYTES};
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);
    input.scene_svg = bytes("<svg/>");
    input.timeline_json = (base_bytes){&byte, PROJECT_BUNDLE_MAX_TIMELINE_BYTES};
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);

    char max_name[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES];
    char max_type[PROJECT_BUNDLE_MAX_MEDIA_TYPE_BYTES];
    memset(max_name, 'n', sizeof(max_name));
    memset(max_type, 't', sizeof(max_type));
    max_type[1] = '/';
    project_bundle_asset longest = {
        {(const uint8_t *)max_name, sizeof(max_name)},
        {(const uint8_t *)max_type, sizeof(max_type)}, bytes("x")
    };
    input.timeline_json = bytes("{}"); input.assets = &longest; input.asset_count = 1U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);

    project_bundle_asset many[PROJECT_BUNDLE_MAX_ASSET_COUNT];
    char names[PROJECT_BUNDLE_MAX_ASSET_COUNT][12];
    for (size_t index = 0U; index < PROJECT_BUNDLE_MAX_ASSET_COUNT; index++) {
        int length = snprintf(names[index], sizeof(names[index]), "asset%04zu", index);
        CHECK(length > 0 && (size_t)length < sizeof(names[index]));
        many[index] = (project_bundle_asset){
            {(const uint8_t *)names[index], (size_t)length}, bytes("image/png"),
            {&byte, 0U}
        };
    }
    input.assets = many; input.asset_count = PROJECT_BUNDLE_MAX_ASSET_COUNT;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);

    project_bundle_asset max_asset_pair[2] = {
        {bytes("large-a"), bytes("application/octet-stream"),
            {&byte, PROJECT_BUNDLE_MAX_ASSET_BYTES}},
        {bytes("large-b"), bytes("application/octet-stream"),
            {&byte, PROJECT_BUNDLE_MAX_ASSET_BYTES}}
    };
    input.assets = max_asset_pair; input.asset_count = 2U;
    CHECK(project_bundle_encoded_size(&input).error == PROJECT_BUNDLE_OK);
    return true;
}

static bool parse_error(const uint8_t *encoded, size_t length, project_bundle_error expected)
{
    project_bundle_view view;
    project_bundle_result parsed = project_bundle_decode((base_bytes){encoded, length}, &view);
    return parsed.error == expected;
}

static bool test_corruption_and_headers(void)
{
    project_bundle_input input = {bytes("{}"), bytes("<svg/>"), bytes("{}"), NULL, 0U};
    uint8_t encoded[128] = {0};
    project_bundle_result write = project_bundle_encode(&input,
        (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(write.error == PROJECT_BUNDLE_OK);
    uint8_t copy[128];
    memcpy(copy, encoded, write.encoded_size);
    copy[0] ^= 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_BAD_MAGIC));
    memcpy(copy, encoded, write.encoded_size);
    copy[8] = 2U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_UNSUPPORTED_VERSION));
    memcpy(copy, encoded, write.encoded_size);
    copy[12] = 55U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[52] = 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[56] ^= 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[58] ^= 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[64] ^= 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[20] = 1U;
    copy[21] = 4U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_LIMIT));
    memcpy(copy, encoded, write.encoded_size);
    memset(copy + 24U, 0xff, 8U);
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_LIMIT));
    memcpy(copy, encoded, write.encoded_size);
    memset(copy + 16U, 0xff, 4U);
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_LIMIT));
    CHECK(parse_error(encoded, write.encoded_size - 1U, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[write.encoded_size] = 0U;
    CHECK(parse_error(copy, write.encoded_size + 1U, PROJECT_BUNDLE_CORRUPT));
    CHECK(project_bundle_decode((base_bytes){encoded, write.encoded_size}, NULL).error ==
          PROJECT_BUNDLE_INVALID_ARGUMENT);
    return true;
}

static bool test_asset_validation(void)
{
    project_bundle_asset asset = {bytes("sprite.bin"), bytes("application/octet-stream"), bytes("pixel")};
    project_bundle_input input = {bytes("{}"), bytes("<svg/>"), bytes("{}"), &asset, 1U};
    uint8_t encoded[256] = {0};
    project_bundle_result write = project_bundle_encode(&input,
        (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(write.error == PROJECT_BUNDLE_OK);
    uint8_t copy[256];
    memcpy(copy, encoded, write.encoded_size);
    copy[write.encoded_size - 1U] ^= 1U;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[write.encoded_size - 20U] = '/';
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[write.encoded_size - asset.data.len - asset.media_type.len - asset.name.len - 16U] = 0xffU;
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[56U + 2U + 6U + 2U + 16U] = 'S';
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    copy[56U + 2U + 6U + 2U + 16U + asset.name.len] = 'I';
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    memcpy(copy, encoded, write.encoded_size);
    memset(copy + write.encoded_size - asset.data.len - 8U, 0xff, 8U);
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    project_bundle_asset pair[2] = {
        {bytes("a.bin"), bytes("image/png"), bytes("a")},
        {bytes("b.bin"), bytes("image/png"), bytes("b")}
    };
    input.assets = pair; input.asset_count = 2U;
    write = project_bundle_encode(&input, (base_mut_bytes){encoded, sizeof(encoded)});
    CHECK(write.error == PROJECT_BUNDLE_OK);
    memcpy(copy, encoded, write.encoded_size);
    size_t first_asset_name = 56U + 2U + 6U + 2U + 16U;
    size_t second_asset_name = first_asset_name + pair[0].name.len +
        pair[0].media_type.len + pair[0].data.len + 16U;
    copy[second_asset_name] = 'a';
    size_t second_record_header = second_asset_name - 16U;
    uint32_t duplicate_crc = test_asset_crc32(
        (base_bytes){copy + second_asset_name, pair[1].name.len},
        (base_bytes){copy + second_asset_name + pair[1].name.len,
                     pair[1].media_type.len},
        (base_bytes){copy + second_asset_name + pair[1].name.len +
                     pair[1].media_type.len, pair[1].data.len});
    for (size_t byte = 0U; byte < 4U; byte++)
        copy[second_record_header + 4U + byte] = (uint8_t)(duplicate_crc >> (byte * 8U));
    CHECK(parse_error(copy, write.encoded_size, PROJECT_BUNDLE_CORRUPT));
    return true;
}

static bool test_decoder_asset_limits(void)
{
    project_bundle_input input = {bytes("{}"), bytes("<svg/>"), bytes("{}"), NULL, 0U};
    uint8_t prefix[128] = {0};
    project_bundle_result encoded_size = project_bundle_encode(&input,
        (base_mut_bytes){prefix, sizeof(prefix)});
    CHECK(encoded_size.error == PROJECT_BUNDLE_OK);
    size_t record_offset = encoded_size.encoded_size;
    uint8_t oversized_record[128] = {0};
    memcpy(oversized_record, prefix, record_offset);
    put_u32(oversized_record + 20U, 1U);
    put_u16(oversized_record + record_offset, 1U);
    put_u16(oversized_record + record_offset + 2U, 3U);
    put_u64(oversized_record + record_offset + 8U,
        (uint64_t)PROJECT_BUNDLE_MAX_ASSET_BYTES + 1U);
    CHECK(parse_error(oversized_record, record_offset + 16U, PROJECT_BUNDLE_LIMIT));

    size_t data_length = PROJECT_BUNDLE_MAX_ASSET_BYTES;
    size_t record_size = 16U + 1U + 3U + data_length;
    size_t total_size = record_offset + record_size * 2U + 16U + 1U + 3U + 1U;
    uint8_t *large = mmap(NULL, total_size, PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANON, -1, 0);
    CHECK(large != MAP_FAILED);
    memcpy(large, prefix, record_offset);
    put_u32(large + 20U, 3U);
    size_t offset = record_offset;
    for (size_t index = 0U; index < 3U; index++) {
        put_u16(large + offset, 1U);
        put_u16(large + offset + 2U, 3U);
        put_u64(large + offset + 8U, index < 2U ? data_length : 1U);
        large[offset + 16U] = (uint8_t)('a' + index);
        memcpy(large + offset + 17U, "x/y", 3U);
        offset += 20U + (index < 2U ? data_length : 1U);
    }
    CHECK(parse_error(large, total_size, PROJECT_BUNDLE_LIMIT));
    CHECK(munmap(large, total_size) == 0);
    return true;
}

static bool test_validated_asset_overlay(void)
{
    static const uint8_t manifest[] = "{}";
    static const uint8_t scene[] = "<svg width='2' height='2'/>";
    static const uint8_t timeline[] = "{\"version\":1,\"tracks\":[]}";
    static const uint8_t svg[] = "<svg width='2' height='2'><path d='M0 0L2 2'/></svg>";
    project_bundle_input input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {scene, sizeof(scene) - 1U},
        .timeline_json = {timeline, sizeof(timeline) - 1U}
    };
    project_bundle_result size = project_bundle_encoded_size(&input);
    CHECK(size.error == PROJECT_BUNDLE_OK);
    uint8_t *encoded = malloc(size.encoded_size);
    CHECK(encoded != NULL && project_bundle_encode(&input,
        (base_mut_bytes){encoded, size.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view view = {0};
    CHECK(project_bundle_decode((base_bytes){encoded, size.encoded_size},
        &view).error == PROJECT_BUNDLE_OK);
    project_bundle_asset_overlay overlay = {0};
    CHECK(project_bundle_asset_overlay_init(&overlay, &view));
    CHECK(!project_bundle_asset_overlay_is_dirty(&overlay));
    CHECK(project_bundle_asset_overlay_add_svg(&overlay, bytes("cel-new"),
        (base_bytes){svg, sizeof(svg) - 1U}).error == PROJECT_BUNDLE_OK);
    CHECK(view.asset_count == 1U &&
        project_bundle_asset_overlay_is_dirty(&overlay));
    project_bundle_asset asset = {0};
    CHECK(project_bundle_asset_at(&view, 0U, &asset) &&
        equal(asset.name, bytes("cel-new")) &&
        equal(asset.media_type, bytes("image/svg+xml")) &&
        equal(asset.data, (base_bytes){svg, sizeof(svg) - 1U}));
    CHECK(project_bundle_asset_overlay_add_svg(&overlay, bytes("cel-new"),
        (base_bytes){svg, sizeof(svg) - 1U}).error ==
        PROJECT_BUNDLE_INVALID_ARGUMENT);
    CHECK(project_bundle_asset_overlay_add_svg(&overlay, bytes("../bad"),
        (base_bytes){svg, sizeof(svg) - 1U}).error ==
        PROJECT_BUNDLE_INVALID_ARGUMENT);
    CHECK(project_bundle_asset_overlay_add_svg(&overlay, bytes("broken"),
        bytes("<svg><script/></svg>")).error == PROJECT_BUNDLE_CORRUPT);
    project_bundle_asset_overlay_mark_saved(&overlay);
    CHECK(!project_bundle_asset_overlay_is_dirty(&overlay));
    project_bundle_asset_overlay_destroy(&overlay);
    CHECK(view.asset_count == 0U && view.overlay_assets == NULL &&
        equal(view.encoded, (base_bytes){encoded, size.encoded_size}));
    free(encoded);
    return true;
}

int main(void)
{
    if (!test_round_trip() || !test_crc_vector_and_empty_timeline() ||
        !test_insufficient_and_invalid_inputs() || !test_exact_limits() ||
        !test_corruption_and_headers() || !test_asset_validation() ||
        !test_validated_asset_overlay() ||
        !test_decoder_asset_limits()) return 1;
    (void)puts("project bundle codec: round-trip, bounds, and corruption checks passed");
    return 0;
}
