#include "project_bundle.h"

#include <stdint.h>
#include <string.h>

#define PROJECT_MAGIC_BYTES 8U
#define PROJECT_ASSET_HEADER_BYTES 16U
#define PROJECT_ASSET_NAME_TABLE_SIZE 2048U

static const uint8_t project_magic[PROJECT_MAGIC_BYTES] = {
    'V', 'E', 'X', 'E', 'L', '\r', '\n', 0x1aU
};

static uint16_t read_u16(const uint8_t *bytes)
{
    return (uint16_t)((uint16_t)bytes[0] | (uint16_t)((uint16_t)bytes[1] << 8U));
}

static uint32_t read_u32(const uint8_t *bytes)
{
    return (uint32_t)bytes[0] | ((uint32_t)bytes[1] << 8U) |
        ((uint32_t)bytes[2] << 16U) | ((uint32_t)bytes[3] << 24U);
}

static uint64_t read_u64(const uint8_t *bytes)
{
    uint64_t value = 0U;
    for (size_t index = 0U; index < 8U; index++)
        value |= (uint64_t)bytes[index] << (index * 8U);
    return value;
}

static void write_u16(uint8_t *bytes, uint16_t value)
{
    bytes[0] = (uint8_t)value;
    bytes[1] = (uint8_t)(value >> 8U);
}

static void write_u32(uint8_t *bytes, uint32_t value)
{
    for (size_t index = 0U; index < 4U; index++)
        bytes[index] = (uint8_t)(value >> (index * 8U));
}

static void write_u64(uint8_t *bytes, uint64_t value)
{
    for (size_t index = 0U; index < 8U; index++)
        bytes[index] = (uint8_t)(value >> (index * 8U));
}

static uint32_t crc32_update(uint32_t crc, base_bytes bytes)
{
    for (size_t index = 0U; index < bytes.len; index++) {
        crc ^= bytes.ptr[index];
        for (unsigned bit = 0U; bit < 8U; bit++) {
            uint32_t mask = (uint32_t)-(int32_t)(crc & 1U);
            crc = (crc >> 1U) ^ (0xedb88320U & mask);
        }
    }
    return crc;
}

static uint32_t crc32_ieee(base_bytes bytes)
{
    return ~crc32_update(UINT32_MAX, bytes);
}

static uint32_t asset_crc32(const project_bundle_asset *asset)
{
    uint32_t crc = UINT32_MAX;
    crc = crc32_update(crc, asset->name);
    crc = crc32_update(crc, asset->media_type);
    crc = crc32_update(crc, asset->data);
    return ~crc;
}

static project_bundle_result result(project_bundle_error error, size_t size,
                                    size_t offset)
{
    return (project_bundle_result){error, size, offset};
}

static bool asset_name_valid(base_bytes name)
{
    if (!base_bytes_valid(name) || name.len == 0U ||
        name.len > PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES ||
        (name.len == 1U && name.ptr[0] == '.') ||
        (name.len == 2U && name.ptr[0] == '.' && name.ptr[1] == '.')) return false;
    for (size_t index = 0U; index < name.len; index++) {
        uint8_t byte = name.ptr[index];
        bool allowed = (byte >= 'a' && byte <= 'z') ||
            (byte >= 'A' && byte <= 'Z') || (byte >= '0' && byte <= '9') ||
            byte == '.' || byte == '_' || byte == '-';
        if (!allowed) return false;
    }
    return true;
}

static bool media_type_valid(base_bytes type)
{
    if (!base_bytes_valid(type) || type.len < 3U ||
        type.len > PROJECT_BUNDLE_MAX_MEDIA_TYPE_BYTES) return false;
    bool slash = false;
    for (size_t index = 0U; index < type.len; index++) {
        uint8_t byte = type.ptr[index];
        if (byte == '/') {
            if (slash || index == 0U || index + 1U == type.len) return false;
            slash = true;
        } else if (byte < 0x21U || byte > 0x7eU) {
            return false;
        }
    }
    return slash;
}

static bool same_bytes(base_bytes left, base_bytes right)
{
    return left.len == right.len &&
        (left.len == 0U || memcmp(left.ptr, right.ptr, left.len) == 0);
}

static size_t asset_name_bucket(base_bytes name)
{
    uint64_t hash = UINT64_C(14695981039346656037);
    for (size_t index = 0U; index < name.len; index++) {
        hash ^= name.ptr[index];
        hash *= UINT64_C(1099511628211);
    }
    return (size_t)(hash & (PROJECT_ASSET_NAME_TABLE_SIZE - 1U));
}

static bool ranges_overlap(const uint8_t *left, size_t left_length,
                           const uint8_t *right, size_t right_length)
{
    if (left_length == 0U || right_length == 0U) return false;
    uintptr_t left_start = (uintptr_t)left;
    uintptr_t right_start = (uintptr_t)right;
    if (left_length > UINTPTR_MAX - left_start ||
        right_length > UINTPTR_MAX - right_start) return true;
    uintptr_t left_end = left_start + left_length;
    uintptr_t right_end = right_start + right_length;
    return left_start < right_end && right_start < left_end;
}

static bool output_overlaps_input(const project_bundle_input *input,
                                  base_mut_bytes output)
{
    size_t asset_descriptors = 0U;
    if (!base_size_mul(input->asset_count, sizeof(*input->assets),
                       &asset_descriptors) ||
        ranges_overlap(output.ptr, output.len, (const uint8_t *)input,
                       sizeof(*input)) ||
        ranges_overlap(output.ptr, output.len,
                       (const uint8_t *)input->assets, asset_descriptors) ||
        ranges_overlap(output.ptr, output.len, input->manifest_json.ptr,
                       input->manifest_json.len) ||
        ranges_overlap(output.ptr, output.len, input->scene_svg.ptr,
                       input->scene_svg.len) ||
        ranges_overlap(output.ptr, output.len, input->timeline_json.ptr,
                       input->timeline_json.len)) return true;
    for (size_t index = 0U; index < input->asset_count; index++) {
        const project_bundle_asset *asset = &input->assets[index];
        if (ranges_overlap(output.ptr, output.len, asset->name.ptr, asset->name.len) ||
            ranges_overlap(output.ptr, output.len, asset->media_type.ptr,
                           asset->media_type.len) ||
            ranges_overlap(output.ptr, output.len, asset->data.ptr, asset->data.len))
            return true;
    }
    return false;
}

static void copy_bytes(uint8_t *destination, base_bytes source)
{
    if (source.len != 0U) memcpy(destination, source.ptr, source.len);
}

static project_bundle_error validate_input(const project_bundle_input *input,
                                           size_t *out_size)
{
    if (input == NULL || out_size == NULL ||
        !base_bytes_valid(input->manifest_json) ||
        !base_bytes_valid(input->scene_svg) ||
        !base_bytes_valid(input->timeline_json) ||
        (input->assets == NULL && input->asset_count != 0U))
        return PROJECT_BUNDLE_INVALID_ARGUMENT;
    if (input->manifest_json.len == 0U || input->scene_svg.len == 0U ||
        input->manifest_json.len > PROJECT_BUNDLE_MAX_MANIFEST_BYTES ||
        input->scene_svg.len > PROJECT_BUNDLE_MAX_SCENE_BYTES ||
        input->timeline_json.len > PROJECT_BUNDLE_MAX_TIMELINE_BYTES ||
        input->asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT)
        return PROJECT_BUNDLE_LIMIT;

    size_t total = PROJECT_BUNDLE_HEADER_BYTES;
    if (!base_size_add(total, input->manifest_json.len, &total) ||
        !base_size_add(total, input->scene_svg.len, &total) ||
        !base_size_add(total, input->timeline_json.len, &total))
        return PROJECT_BUNDLE_LIMIT;
    size_t asset_total = 0U;
    for (size_t index = 0U; index < input->asset_count; index++) {
        const project_bundle_asset *asset = &input->assets[index];
        if (!base_bytes_valid(asset->name) || !base_bytes_valid(asset->media_type) ||
            !base_bytes_valid(asset->data)) return PROJECT_BUNDLE_INVALID_ARGUMENT;
        if (!asset_name_valid(asset->name) || !media_type_valid(asset->media_type))
            return PROJECT_BUNDLE_INVALID_ARGUMENT;
        if (asset->data.len > PROJECT_BUNDLE_MAX_ASSET_BYTES ||
            !base_size_add(asset_total, asset->data.len, &asset_total) ||
            asset_total > PROJECT_BUNDLE_MAX_ASSET_TOTAL_BYTES)
            return PROJECT_BUNDLE_LIMIT;
        for (size_t prior = 0U; prior < index; prior++) {
            if (same_bytes(asset->name, input->assets[prior].name))
                return PROJECT_BUNDLE_INVALID_ARGUMENT;
        }
        size_t record_size = PROJECT_ASSET_HEADER_BYTES;
        if (!base_size_add(record_size, asset->name.len, &record_size) ||
            !base_size_add(record_size, asset->media_type.len, &record_size) ||
            !base_size_add(record_size, asset->data.len, &record_size) ||
            !base_size_add(total, record_size, &total)) return PROJECT_BUNDLE_LIMIT;
    }
    if (total > PROJECT_BUNDLE_MAX_BYTES) return PROJECT_BUNDLE_LIMIT;
    *out_size = total;
    return PROJECT_BUNDLE_OK;
}

project_bundle_result project_bundle_encoded_size(const project_bundle_input *input)
{
    size_t size = 0U;
    project_bundle_error error = validate_input(input, &size);
    return result(error, size, 0U);
}

project_bundle_result project_bundle_encode(const project_bundle_input *input,
                                            base_mut_bytes output)
{
    if (!base_mut_bytes_valid(output))
        return result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U, 0U);
    size_t size = 0U;
    project_bundle_error error = validate_input(input, &size);
    if (error != PROJECT_BUNDLE_OK) return result(error, 0U, 0U);
    if (output.len < size) return result(PROJECT_BUNDLE_BUFFER_TOO_SMALL, size, 0U);
    if (output_overlaps_input(input, output))
        return result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U, 0U);

    uint8_t *bytes = output.ptr;
    memcpy(bytes, project_magic, sizeof(project_magic));
    write_u32(bytes + 8U, PROJECT_BUNDLE_VERSION);
    write_u32(bytes + 12U, PROJECT_BUNDLE_HEADER_BYTES);
    write_u32(bytes + 16U, (uint32_t)input->manifest_json.len);
    write_u32(bytes + 20U, (uint32_t)input->asset_count);
    write_u64(bytes + 24U, (uint64_t)input->scene_svg.len);
    write_u64(bytes + 32U, (uint64_t)input->timeline_json.len);
    write_u32(bytes + 40U, crc32_ieee(input->manifest_json));
    write_u32(bytes + 44U, crc32_ieee(input->scene_svg));
    write_u32(bytes + 48U, crc32_ieee(input->timeline_json));
    write_u32(bytes + 52U, 0U);

    size_t offset = PROJECT_BUNDLE_HEADER_BYTES;
    copy_bytes(bytes + offset, input->manifest_json);
    offset += input->manifest_json.len;
    copy_bytes(bytes + offset, input->scene_svg);
    offset += input->scene_svg.len;
    copy_bytes(bytes + offset, input->timeline_json);
    offset += input->timeline_json.len;
    for (size_t index = 0U; index < input->asset_count; index++) {
        const project_bundle_asset *asset = &input->assets[index];
        write_u16(bytes + offset, (uint16_t)asset->name.len);
        write_u16(bytes + offset + 2U, (uint16_t)asset->media_type.len);
        write_u32(bytes + offset + 4U, asset_crc32(asset));
        write_u64(bytes + offset + 8U, (uint64_t)asset->data.len);
        offset += PROJECT_ASSET_HEADER_BYTES;
        copy_bytes(bytes + offset, asset->name);
        offset += asset->name.len;
        copy_bytes(bytes + offset, asset->media_type);
        offset += asset->media_type.len;
        copy_bytes(bytes + offset, asset->data);
        offset += asset->data.len;
    }
    return result(PROJECT_BUNDLE_OK, offset, 0U);
}

static bool take_slice(base_bytes encoded, size_t *offset, size_t length,
                       base_bytes *out)
{
    if (offset == NULL || out == NULL || *offset > encoded.len ||
        length > encoded.len - *offset) return false;
    *out = (base_bytes){encoded.ptr + *offset, length};
    *offset += length;
    return true;
}

static bool decode_asset_record(base_bytes encoded, size_t *offset,
                                project_bundle_asset *out)
{
    if (*offset > encoded.len ||
        PROJECT_ASSET_HEADER_BYTES > encoded.len - *offset) return false;
    const uint8_t *header = encoded.ptr + *offset;
    size_t name_length = read_u16(header);
    size_t media_length = read_u16(header + 2U);
    uint32_t expected_crc = read_u32(header + 4U);
    uint64_t data_length64 = read_u64(header + 8U);
    if (data_length64 > SIZE_MAX) return false;
    size_t data_length = (size_t)data_length64;
    *offset += PROJECT_ASSET_HEADER_BYTES;
    if (!take_slice(encoded, offset, name_length, &out->name) ||
        !take_slice(encoded, offset, media_length, &out->media_type) ||
        !take_slice(encoded, offset, data_length, &out->data) ||
        !asset_name_valid(out->name) || !media_type_valid(out->media_type) ||
        data_length > PROJECT_BUNDLE_MAX_ASSET_BYTES ||
        asset_crc32(out) != expected_crc) return false;
    return true;
}

static project_bundle_result preflight_asset_records(base_bytes encoded,
                                                     size_t offset,
                                                     size_t count)
{
    size_t total_data = 0U;
    for (size_t index = 0U; index < count; index++) {
        if (offset > encoded.len ||
            PROJECT_ASSET_HEADER_BYTES > encoded.len - offset)
            return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
        const uint8_t *header = encoded.ptr + offset;
        size_t name_length = read_u16(header);
        size_t media_length = read_u16(header + 2U);
        uint64_t data_length64 = read_u64(header + 8U);
        if (data_length64 > SIZE_MAX || data_length64 > PROJECT_BUNDLE_MAX_ASSET_BYTES)
            return result(PROJECT_BUNDLE_LIMIT, 0U, offset + 8U);
        size_t data_length = (size_t)data_length64;
        if (!base_size_add(total_data, data_length, &total_data) ||
            total_data > PROJECT_BUNDLE_MAX_ASSET_TOTAL_BYTES)
            return result(PROJECT_BUNDLE_LIMIT, 0U, offset + 8U);
        size_t record_size = PROJECT_ASSET_HEADER_BYTES;
        if (!base_size_add(record_size, name_length, &record_size) ||
            !base_size_add(record_size, media_length, &record_size) ||
            !base_size_add(record_size, data_length, &record_size) ||
            record_size > encoded.len - offset)
            return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
        base_bytes name = {header + PROJECT_ASSET_HEADER_BYTES, name_length};
        base_bytes media = {name.ptr + name.len, media_length};
        if (!asset_name_valid(name) || !media_type_valid(media))
            return result(PROJECT_BUNDLE_CORRUPT, 0U, offset + PROJECT_ASSET_HEADER_BYTES);
        offset += record_size;
    }
    if (offset != encoded.len) return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
    return result(PROJECT_BUNDLE_OK, encoded.len, 0U);
}

project_bundle_result project_bundle_decode(base_bytes encoded,
                                            project_bundle_view *out_view)
{
    if (!base_bytes_valid(encoded) || out_view == NULL)
        return result(PROJECT_BUNDLE_INVALID_ARGUMENT, 0U, 0U);
    *out_view = (project_bundle_view){0};
    if (encoded.len < PROJECT_BUNDLE_HEADER_BYTES ||
        encoded.len > PROJECT_BUNDLE_MAX_BYTES)
        return result(PROJECT_BUNDLE_CORRUPT, 0U, encoded.len);
    if (memcmp(encoded.ptr, project_magic, sizeof(project_magic)) != 0)
        return result(PROJECT_BUNDLE_BAD_MAGIC, 0U, 0U);
    uint32_t version = read_u32(encoded.ptr + 8U);
    if (version != PROJECT_BUNDLE_VERSION)
        return result(PROJECT_BUNDLE_UNSUPPORTED_VERSION, 0U, 8U);
    if (read_u32(encoded.ptr + 12U) != PROJECT_BUNDLE_HEADER_BYTES ||
        read_u32(encoded.ptr + 52U) != 0U)
        return result(PROJECT_BUNDLE_CORRUPT, 0U, 12U);

    size_t manifest_length = read_u32(encoded.ptr + 16U);
    size_t asset_count = read_u32(encoded.ptr + 20U);
    uint64_t scene_length64 = read_u64(encoded.ptr + 24U);
    uint64_t timeline_length64 = read_u64(encoded.ptr + 32U);
    if (scene_length64 > SIZE_MAX || timeline_length64 > SIZE_MAX ||
        manifest_length > PROJECT_BUNDLE_MAX_MANIFEST_BYTES ||
        scene_length64 == 0U || scene_length64 > PROJECT_BUNDLE_MAX_SCENE_BYTES ||
        timeline_length64 > PROJECT_BUNDLE_MAX_TIMELINE_BYTES ||
        asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT)
        return result(PROJECT_BUNDLE_LIMIT, 0U, 16U);

    size_t offset = PROJECT_BUNDLE_HEADER_BYTES;
    project_bundle_view view = {0};
    if (!take_slice(encoded, &offset, manifest_length, &view.manifest_json) ||
        !take_slice(encoded, &offset, (size_t)scene_length64, &view.scene_svg) ||
        !take_slice(encoded, &offset, (size_t)timeline_length64, &view.timeline_json))
        return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
    if (manifest_length == 0U ||
        crc32_ieee(view.manifest_json) != read_u32(encoded.ptr + 40U) ||
        crc32_ieee(view.scene_svg) != read_u32(encoded.ptr + 44U) ||
        crc32_ieee(view.timeline_json) != read_u32(encoded.ptr + 48U))
        return result(PROJECT_BUNDLE_CORRUPT, 0U, PROJECT_BUNDLE_HEADER_BYTES);

    size_t asset_start = offset;
    project_bundle_result preflight = preflight_asset_records(encoded, asset_start,
                                                               asset_count);
    if (preflight.error != PROJECT_BUNDLE_OK) return preflight;
    base_bytes name_table[PROJECT_ASSET_NAME_TABLE_SIZE] = {{0}};
    for (size_t index = 0U; index < asset_count; index++) {
        project_bundle_asset current;
        if (!decode_asset_record(encoded, &offset, &current))
            return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
        size_t bucket = asset_name_bucket(current.name);
        size_t probes = 0U;
        while (name_table[bucket].len != 0U &&
               !same_bytes(current.name, name_table[bucket])) {
            bucket = (bucket + 1U) & (PROJECT_ASSET_NAME_TABLE_SIZE - 1U);
            probes++;
        }
        if (name_table[bucket].len != 0U || probes >= PROJECT_ASSET_NAME_TABLE_SIZE)
            return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
        name_table[bucket] = current.name;
    }
    if (offset != encoded.len) return result(PROJECT_BUNDLE_CORRUPT, 0U, offset);
    view.version = version;
    view.encoded = encoded;
    view.asset_count = asset_count;
    view.base_asset_count = asset_count;
    view.overlay_assets = NULL;
    *out_view = view;
    return result(PROJECT_BUNDLE_OK, encoded.len, 0U);
}

bool project_bundle_has_magic(base_bytes encoded)
{
    return base_bytes_valid(encoded) && encoded.len >= PROJECT_MAGIC_BYTES &&
        memcmp(encoded.ptr, project_magic, PROJECT_MAGIC_BYTES) == 0;
}

bool project_bundle_asset_at(const project_bundle_view *view, size_t index,
                             project_bundle_asset *out_asset)
{
    if (view == NULL || out_asset == NULL || index >= view->asset_count)
        return false;
    if (view->overlay_assets != NULL && index >= view->base_asset_count) {
        size_t overlay_index = index - view->base_asset_count;
        size_t overlay_count = view->asset_count - view->base_asset_count;
        if (overlay_index >= overlay_count) return false;
        *out_asset = view->overlay_assets[overlay_index];
        return true;
    }
    if (
        !base_bytes_valid(view->encoded)) return false;
    size_t offset = PROJECT_BUNDLE_HEADER_BYTES;
    size_t scene_length64_offset = 24U;
    if (view->encoded.len < PROJECT_BUNDLE_HEADER_BYTES) return false;
    uint64_t scene_length64 = read_u64(view->encoded.ptr + scene_length64_offset);
    uint64_t timeline_length64 = read_u64(view->encoded.ptr + 32U);
    size_t manifest_length = read_u32(view->encoded.ptr + 16U);
    if (scene_length64 > SIZE_MAX || timeline_length64 > SIZE_MAX ||
        !base_size_add(offset, manifest_length, &offset) ||
        !base_size_add(offset, (size_t)scene_length64, &offset) ||
        !base_size_add(offset, (size_t)timeline_length64, &offset) ||
        offset > view->encoded.len) return false;
    for (size_t current = 0U; current < index; current++) {
        if (offset > view->encoded.len ||
            PROJECT_ASSET_HEADER_BYTES > view->encoded.len - offset) return false;
        const uint8_t *header = view->encoded.ptr + offset;
        size_t name_length = read_u16(header);
        size_t media_length = read_u16(header + 2U);
        uint64_t data_length64 = read_u64(header + 8U);
        if (data_length64 > SIZE_MAX) return false;
        size_t record_size = PROJECT_ASSET_HEADER_BYTES;
        if (!base_size_add(record_size, name_length, &record_size) ||
            !base_size_add(record_size, media_length, &record_size) ||
            !base_size_add(record_size, (size_t)data_length64, &record_size) ||
            record_size > view->encoded.len - offset) return false;
        offset += record_size;
    }
    return decode_asset_record(view->encoded, &offset, out_asset);
}

const char *project_bundle_error_name(project_bundle_error error)
{
    switch (error) {
        case PROJECT_BUNDLE_OK: return "ok";
        case PROJECT_BUNDLE_INVALID_ARGUMENT: return "invalid_argument";
        case PROJECT_BUNDLE_LIMIT: return "limit";
        case PROJECT_BUNDLE_BUFFER_TOO_SMALL: return "buffer_too_small";
        case PROJECT_BUNDLE_BAD_MAGIC: return "bad_magic";
        case PROJECT_BUNDLE_UNSUPPORTED_VERSION: return "unsupported_version";
        case PROJECT_BUNDLE_CORRUPT: return "corrupt";
        case PROJECT_BUNDLE_OUT_OF_MEMORY: return "out_of_memory";
    }
    return "unknown";
}
