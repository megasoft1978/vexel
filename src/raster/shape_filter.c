#include "shape_filter.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define FILTER_SCRATCH_LIMIT (64U * 1024U * 1024U)
#define FILTER_WORK_LIMIT 50000000U

typedef struct filter_surfaces {
    raster_tile graphic;
    raster_tile alpha;
    raster_tile results[SVG_SHAPE_MAX_FILTER_PRIMITIVES];
    bool has_alpha;
    bool has_drop_shadow;
    float *blur_work;
    uint8_t *shadow_alpha;
    size_t scratch_bytes;
} filter_surfaces;

static bool add_size(size_t *total, size_t amount)
{
    if (amount > SIZE_MAX - *total) return false;
    *total += amount;
    return true;
}

static bool multiply_size(size_t left, size_t right, size_t *out)
{
    if (left != 0U && right > SIZE_MAX / left) return false;
    *out = left * right;
    return true;
}

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error error,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){error, offset};
}

static double clamp01(double value)
{
    if (!isfinite(value) || value < 0.0) return 0.0;
    return value > 1.0 ? 1.0 : value;
}

static double decode_srgb(uint8_t value)
{
    double sample = (double)value / 255.0;
    return sample <= 0.04045 ? sample / 12.92 :
           pow((sample + 0.055) / 1.055, 2.4);
}

static uint8_t encode_srgb(double value)
{
    value = clamp01(value);
    double sample = value <= 0.0031308 ? 12.92 * value :
                    1.055 * pow(value, 1.0 / 2.4) - 0.055;
    return (uint8_t)floor(sample * 255.0 + 0.5);
}

static bool create_surface(uint32_t width, uint32_t height,
                           raster_tile *surface)
{
    if (!raster_tile_create(width, height, surface)) return false;
    memset(surface->rgba, 0, surface->byte_count);
    return true;
}

static void destroy_surfaces(filter_surfaces *surfaces)
{
    raster_tile_destroy(&surfaces->graphic);
    raster_tile_destroy(&surfaces->alpha);
    for (size_t i = 0U; i < SVG_SHAPE_MAX_FILTER_PRIMITIVES; ++i)
        raster_tile_destroy(&surfaces->results[i]);
    free(surfaces->blur_work);
    free(surfaces->shadow_alpha);
    *surfaces = (filter_surfaces){0};
}

static bool input_is_alpha(const svg_shape_filter_primitive *primitive)
{
    if (primitive->input == SVG_SHAPE_FILTER_INPUT_ALPHA ||
        primitive->input2 == SVG_SHAPE_FILTER_INPUT_ALPHA) return true;
    for (size_t i = 0U; i < primitive->merge_count; ++i)
        if (primitive->merge_inputs[i] == SVG_SHAPE_FILTER_INPUT_ALPHA)
            return true;
    return false;
}

static const uint8_t *resolve_input(const filter_surfaces *surfaces,
                                   const svg_shape_filter_primitive *primitive,
                                   size_t primitive_index, int32_t input)
{
    if (input == SVG_SHAPE_FILTER_INPUT_PREVIOUS)
        input = primitive_index == 0U ? SVG_SHAPE_FILTER_INPUT_GRAPHIC :
                (int32_t)(primitive_index - 1U);
    if (input == SVG_SHAPE_FILTER_INPUT_GRAPHIC)
        return surfaces->graphic.rgba;
    if (input == SVG_SHAPE_FILTER_INPUT_ALPHA)
        return surfaces->alpha.rgba;
    if (input < 0 || (size_t)input >= primitive_index) return NULL;
    (void)primitive;
    return surfaces->results[(size_t)input].rgba;
}

static bool filter_work(const svg_shape_filter_primitive *primitives,
                        size_t primitive_count, size_t pixels, size_t *work)
{
    size_t total = 0U;
    for (size_t i = 0U; i < primitive_count; ++i) {
        const svg_shape_filter_primitive *primitive = &primitives[i];
        size_t per_pixel = 1U;
        if (primitive->kind == SVG_SHAPE_FILTER_GAUSSIAN_BLUR ||
            primitive->kind == SVG_SHAPE_FILTER_DROP_SHADOW) {
            double rx_value = ceil(3.0 * primitive->values[0]);
            double ry_value = ceil(3.0 * primitive->values[1]);
            if (!isfinite(rx_value) || !isfinite(ry_value) ||
                rx_value > 96.0 || ry_value > 96.0) return false;
            per_pixel = (size_t)(2.0 * rx_value + 1.0) +
                        (size_t)(2.0 * ry_value + 1.0);
        } else if (primitive->kind == SVG_SHAPE_FILTER_MERGE) {
            per_pixel = primitive->merge_count;
        }
        size_t amount = 0U;
        if (!multiply_size(pixels, per_pixel, &amount) ||
            !add_size(&total, amount) || total > FILTER_WORK_LIMIT) return false;
    }
    *work = total;
    return true;
}

static void source_over(uint8_t destination[4], const uint8_t source[4])
{
    double source_alpha = (double)source[3] / 255.0;
    double backdrop_alpha = (double)destination[3] / 255.0;
    double output_alpha = source_alpha + backdrop_alpha -
                          source_alpha * backdrop_alpha;
    if (output_alpha <= 0.0) {
        memset(destination, 0, 4U);
        return;
    }
    for (size_t channel = 0U; channel < 3U; ++channel) {
        double foreground = (double)source[channel] / 255.0;
        double backdrop = (double)destination[channel] / 255.0;
        double premultiplied = foreground * source_alpha +
            backdrop * backdrop_alpha * (1.0 - source_alpha);
        destination[channel] = (uint8_t)floor(
            clamp01(premultiplied / output_alpha) * 255.0 + 0.5);
    }
    destination[3] = (uint8_t)floor(output_alpha * 255.0 + 0.5);
}

static double blend_component(double source, double backdrop, uint8_t mode)
{
    switch (mode) {
    case 1U: return source * backdrop;
    case 2U: return source + backdrop - source * backdrop;
    case 3U: return source < backdrop ? source : backdrop;
    case 4U: return source > backdrop ? source : backdrop;
    default: return source;
    }
}

static void blend_pixel(uint8_t output[4], const uint8_t *source,
                        const uint8_t *backdrop, uint8_t mode)
{
    double sa = (double)source[3] / 255.0;
    double ba = (double)backdrop[3] / 255.0;
    double oa = sa + ba - sa * ba;
    if (oa <= 0.0) {
        memset(output, 0, 4U);
        return;
    }
    for (size_t c = 0U; c < 3U; ++c) {
        double sc = (double)source[c] / 255.0;
        double bc = (double)backdrop[c] / 255.0;
        double mixed = blend_component(sc, bc, mode);
        double premultiplied = sc * sa * (1.0 - ba) +
            bc * ba * (1.0 - sa) + mixed * sa * ba;
        output[c] = (uint8_t)floor(clamp01(premultiplied / oa) * 255.0 + 0.5);
    }
    output[3] = (uint8_t)floor(oa * 255.0 + 0.5);
}

static bool blur(const uint8_t *input, uint8_t *output,
                 uint32_t width, uint32_t height,
                 double sigma_x, double sigma_y,
                 float *work)
{
    if (input == NULL || output == NULL || work == NULL) return false;
    size_t pixels = (size_t)width * (size_t)height;
    double radius_x_value = ceil(3.0 * sigma_x);
    double radius_y_value = ceil(3.0 * sigma_y);
    if (radius_x_value > 96.0 || radius_y_value > 96.0) return false;
    int radius_x = (int)radius_x_value;
    int radius_y = (int)radius_y_value;
    double weights_x[193] = {0.0};
    double weights_y[193] = {0.0};
    double total_x = 0.0, total_y = 0.0;
    for (int k = -radius_x; k <= radius_x; ++k) {
        double weight = sigma_x == 0.0 ? 1.0 :
            exp(-((double)k * (double)k) / (2.0 * sigma_x * sigma_x));
        weights_x[k + radius_x] = weight;
        total_x += weight;
    }
    for (int k = -radius_y; k <= radius_y; ++k) {
        double weight = sigma_y == 0.0 ? 1.0 :
            exp(-((double)k * (double)k) / (2.0 * sigma_y * sigma_y));
        weights_y[k + radius_y] = weight;
        total_y += weight;
    }
    if (total_x <= 0.0 || total_y <= 0.0) return false;
    for (uint32_t y = 0U; y < height; ++y) {
        for (uint32_t x = 0U; x < width; ++x) {
            double sum[4] = {0.0, 0.0, 0.0, 0.0};
            for (int k = -radius_x; k <= radius_x; ++k) {
                int64_t sx = (int64_t)x + k;
                if (sx < 0 || sx >= (int64_t)width) continue;
                const uint8_t *pixel = input +
                    ((size_t)y * width + (size_t)sx) * 4U;
                double alpha = (double)pixel[3] / 255.0;
                double weight = weights_x[k + radius_x] / total_x;
                sum[0] += (double)pixel[0] / 255.0 * alpha * weight;
                sum[1] += (double)pixel[1] / 255.0 * alpha * weight;
                sum[2] += (double)pixel[2] / 255.0 * alpha * weight;
                sum[3] += alpha * weight;
            }
            size_t index = ((size_t)y * width + x) * 4U;
            for (size_t c = 0U; c < 4U; ++c) work[index + c] = (float)sum[c];
        }
    }
    for (uint32_t y = 0U; y < height; ++y) {
        for (uint32_t x = 0U; x < width; ++x) {
            double sum[4] = {0.0, 0.0, 0.0, 0.0};
            for (int k = -radius_y; k <= radius_y; ++k) {
                int64_t sy = (int64_t)y + k;
                if (sy < 0 || sy >= (int64_t)height) continue;
                size_t index = ((size_t)sy * width + x) * 4U;
                double weight = weights_y[k + radius_y] / total_y;
                for (size_t c = 0U; c < 4U; ++c)
                    sum[c] += (double)work[index + c] * weight;
            }
            double alpha = clamp01(sum[3]);
            size_t index = ((size_t)y * width + x) * 4U;
            if (alpha <= 0.0) {
                memset(output + index, 0, 4U);
                continue;
            }
            for (size_t c = 0U; c < 3U; ++c)
                output[index + c] = (uint8_t)floor(
                    clamp01(sum[c] / alpha) * 255.0 + 0.5);
            output[index + 3U] = (uint8_t)floor(alpha * 255.0 + 0.5);
        }
    }
    (void)pixels;
    return true;
}

static void offset_surface(const uint8_t *input, uint8_t *output,
                           uint32_t width, uint32_t height,
                           double dx, double dy)
{
    int64_t shift_x = (int64_t)llround(dx);
    int64_t shift_y = (int64_t)llround(dy);
    memset(output, 0, (size_t)width * height * 4U);
    for (uint32_t y = 0U; y < height; ++y) {
        int64_t destination_y = (int64_t)y + shift_y;
        if (destination_y < 0 || destination_y >= (int64_t)height) continue;
        for (uint32_t x = 0U; x < width; ++x) {
            int64_t destination_x = (int64_t)x + shift_x;
            if (destination_x < 0 || destination_x >= (int64_t)width) continue;
            memcpy(output + ((size_t)destination_y * width +
                             (size_t)destination_x) * 4U,
                   input + ((size_t)y * width + x) * 4U, 4U);
        }
    }
}

static void color_matrix(const svg_shape_filter_primitive *primitive,
                         const uint8_t *input, uint8_t *output, size_t pixels)
{
    static const double lum[3] = {0.2126, 0.7152, 0.0722};
    double matrix[20] = {0.0};
    if (primitive->mode == 0U) {
        memcpy(matrix, primitive->values, sizeof(matrix));
    } else if (primitive->mode == 1U) {
        double s = primitive->values[0];
        matrix[0] = 0.213 + 0.787 * s;
        matrix[1] = 0.715 - 0.715 * s;
        matrix[2] = 0.072 - 0.072 * s;
        matrix[5] = 0.213 - 0.213 * s;
        matrix[6] = 0.715 + 0.285 * s;
        matrix[7] = 0.072 - 0.072 * s;
        matrix[10] = 0.213 - 0.213 * s;
        matrix[11] = 0.715 - 0.715 * s;
        matrix[12] = 0.072 + 0.928 * s;
        matrix[18] = 1.0;
    } else if (primitive->mode == 2U) {
        double angle = primitive->values[0] * (3.14159265358979323846 / 180.0);
        double c = cos(angle), s = sin(angle);
        matrix[0] = 0.213 + c * 0.787 - s * 0.213;
        matrix[1] = 0.715 - c * 0.715 - s * 0.715;
        matrix[2] = 0.072 - c * 0.072 + s * 0.928;
        matrix[5] = 0.213 - c * 0.213 + s * 0.143;
        matrix[6] = 0.715 + c * 0.285 + s * 0.140;
        matrix[7] = 0.072 - c * 0.072 - s * 0.283;
        matrix[10] = 0.213 - c * 0.213 - s * 0.787;
        matrix[11] = 0.715 - c * 0.715 + s * 0.715;
        matrix[12] = 0.072 + c * 0.928 + s * 0.072;
        matrix[18] = 1.0;
    } else {
        matrix[15] = lum[0];
        matrix[16] = lum[1];
        matrix[17] = lum[2];
    }
    for (size_t pixel = 0U; pixel < pixels; ++pixel) {
        const uint8_t *source = input + pixel * 4U;
        uint8_t *destination = output + pixel * 4U;
        double rgba[4] = {(double)source[0] / 255.0,
                          (double)source[1] / 255.0,
                          (double)source[2] / 255.0,
                          (double)source[3] / 255.0};
        if (primitive->mode == 3U) {
            destination[0] = destination[1] = destination[2] = 0U;
            destination[3] = (uint8_t)floor(clamp01(
                lum[0] * rgba[0] + lum[1] * rgba[1] + lum[2] * rgba[2]) *
                255.0 + 0.5);
            continue;
        }
        if (source[3] == 0U) {
            memset(destination, 0, 4U);
            continue;
        }
        for (size_t row = 0U; row < 4U; ++row) {
            double value = matrix[row * 5U + 4U];
            for (size_t column = 0U; column < 4U; ++column)
                value += matrix[row * 5U + column] * rgba[column];
            destination[row] = (uint8_t)floor(clamp01(value) * 255.0 + 0.5);
        }
    }
}

static void composite(const svg_shape_filter_primitive *primitive,
                      const uint8_t *in, const uint8_t *in2,
                      uint8_t *output, size_t pixels)
{
    for (size_t pixel = 0U; pixel < pixels; ++pixel) {
        const uint8_t *a = in + pixel * 4U;
        const uint8_t *b = in2 + pixel * 4U;
        uint8_t *out = output + pixel * 4U;
        if (primitive->kind == SVG_SHAPE_FILTER_BLEND) {
            blend_pixel(out, a, b, primitive->mode);
            continue;
        }
        double alpha_a = (double)a[3] / 255.0;
        double alpha_b = (double)b[3] / 255.0;
        if (primitive->mode == 6U) {
            for (size_t c = 0U; c < 4U; ++c) {
                double av = (double)a[c] / 255.0;
                double bv = (double)b[c] / 255.0;
                double value = primitive->values[0] * av * bv +
                    primitive->values[1] * av +
                    primitive->values[2] * bv + primitive->values[3];
                out[c] = (uint8_t)floor(clamp01(value) * 255.0 + 0.5);
            }
            continue;
        }
        double alpha = 0.0;
        if (primitive->mode == 0U) alpha = alpha_a + alpha_b * (1.0 - alpha_a);
        else if (primitive->mode == 1U) alpha = alpha_a * alpha_b;
        else if (primitive->mode == 2U) alpha = alpha_a * (1.0 - alpha_b);
        else if (primitive->mode == 3U) alpha = alpha_b;
        else if (primitive->mode == 4U)
            alpha = alpha_a * (1.0 - alpha_b) + alpha_b * (1.0 - alpha_a);
        else alpha = fmin(1.0, alpha_a + alpha_b);
        if (alpha <= 0.0) {
            memset(out, 0, 4U);
            continue;
        }
        for (size_t c = 0U; c < 3U; ++c) {
            double ca = (double)a[c] / 255.0;
            double cb = (double)b[c] / 255.0;
            double premultiplied = 0.0;
            if (primitive->mode == 0U)
                premultiplied = ca * alpha_a + cb * alpha_b * (1.0 - alpha_a);
            else if (primitive->mode == 1U)
                premultiplied = ca * alpha_a * alpha_b;
            else if (primitive->mode == 2U)
                premultiplied = ca * alpha_a * (1.0 - alpha_b);
            else if (primitive->mode == 3U)
                premultiplied = ca * alpha_a * alpha_b +
                    cb * alpha_b * (1.0 - alpha_a);
            else if (primitive->mode == 4U)
                premultiplied = ca * alpha_a * (1.0 - alpha_b) +
                    cb * alpha_b * (1.0 - alpha_a);
            else premultiplied = fmin(alpha, ca * alpha_a + cb * alpha_b);
            out[c] = (uint8_t)floor(clamp01(premultiplied / alpha) *
                                    255.0 + 0.5);
        }
        out[3] = (uint8_t)floor(clamp01(alpha) * 255.0 + 0.5);
    }
}

static bool drop_shadow(const svg_shape_filter_primitive *primitive,
                        const uint8_t *input, uint8_t *output,
                        uint8_t *shadow_alpha, float *work,
                        uint32_t width, uint32_t height, bool linear_rgb)
{
    size_t pixels = (size_t)width * height;
    if (!blur(input, output, width, height, primitive->values[0],
              primitive->values[1], work)) return false;
    for (size_t pixel = 0U; pixel < pixels; ++pixel)
        shadow_alpha[pixel] = output[pixel * 4U + 3U];
    double color[3];
    for (size_t c = 0U; c < 3U; ++c) {
        color[c] = (double)primitive->rgba[c] / 255.0;
        if (linear_rgb) color[c] = decode_srgb(primitive->rgba[c]);
    }
    double color_alpha = (double)primitive->rgba[3] / 255.0;
    int64_t dx = (int64_t)llround(primitive->values[2]);
    int64_t dy = (int64_t)llround(primitive->values[3]);
    memset(output, 0, pixels * 4U);
    for (uint32_t y = 0U; y < height; ++y) {
        for (uint32_t x = 0U; x < width; ++x) {
            int64_t sx = (int64_t)x - dx;
            int64_t sy = (int64_t)y - dy;
            if (sx < 0 || sy < 0 || sx >= (int64_t)width ||
                sy >= (int64_t)height) continue;
            size_t source_pixel = (size_t)sy * width + (size_t)sx;
            size_t destination = ((size_t)y * width + x) * 4U;
            memcpy(output + destination, primitive->rgba, 4U);
            output[destination + 3U] = (uint8_t)floor(
                ((double)shadow_alpha[source_pixel] / 255.0) *
                color_alpha * 255.0 + 0.5);
            for (size_t c = 0U; c < 3U; ++c)
                output[destination + c] = (uint8_t)floor(
                    clamp01(color[c]) * 255.0 + 0.5);
        }
    }
    for (size_t pixel = 0U; pixel < pixels; ++pixel)
        source_over(output + pixel * 4U, input + pixel * 4U);
    return true;
}

static bool viewport_mapping(const svg_shape_document *document,
                            const raster_tile *tile, double *scale_x,
                            double *scale_y, double *offset_x,
                            double *offset_y)
{
    *scale_x = (double)tile->width / document->view_width;
    *scale_y = (double)tile->height / document->view_height;
    *offset_x = 0.0;
    *offset_y = 0.0;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(*scale_x, *scale_y) :
                                      fmin(*scale_x, *scale_y);
        *scale_x = scale;
        *scale_y = scale;
        *offset_x = ((double)tile->width - document->view_width * scale) *
                    aspect.align_x;
        *offset_y = ((double)tile->height - document->view_height * scale) *
                    aspect.align_y;
    }
    return isfinite(*scale_x) && isfinite(*scale_y) &&
           isfinite(*offset_x) && isfinite(*offset_y) &&
           *scale_x > 0.0 && *scale_y > 0.0;
}

static void find_alpha_bounds(const uint8_t *rgba, uint32_t width,
                              uint32_t height, double bounds[4],
                              bool *found)
{
    uint32_t min_x = width, min_y = height, max_x = 0U, max_y = 0U;
    *found = false;
    for (uint32_t y = 0U; y < height; ++y) {
        for (uint32_t x = 0U; x < width; ++x) {
            if (rgba[((size_t)y * width + x) * 4U + 3U] == 0U) continue;
            if (x < min_x) min_x = x;
            if (y < min_y) min_y = y;
            if (x + 1U > max_x) max_x = x + 1U;
            if (y + 1U > max_y) max_y = y + 1U;
            *found = true;
        }
    }
    if (*found) {
        bounds[0] = (double)min_x;
        bounds[1] = (double)min_y;
        bounds[2] = (double)max_x;
        bounds[3] = (double)max_y;
    } else {
        bounds[0] = bounds[1] = bounds[2] = bounds[3] = 0.0;
    }
}

static void clip_filter_region(const svg_shape_document *document,
                               const svg_shape_filter *filter,
                               raster_tile *tile,
                               const double source_bounds[4],
                               bool have_source_bounds,
                               double scale_x, double scale_y,
                               double offset_x, double offset_y)
{
    if (!have_source_bounds) {
        memset(tile->rgba, 0, tile->byte_count);
        return;
    }
    double region[4];
    if (!filter->custom_region ||
        filter->filter_units_object_bounding_box) {
        double width = source_bounds[2] - source_bounds[0];
        double height = source_bounds[3] - source_bounds[1];
        region[0] = source_bounds[0] + filter->region[0] * width;
        region[1] = source_bounds[1] + filter->region[1] * height;
        region[2] = region[0] + filter->region[2] * width;
        region[3] = region[1] + filter->region[3] * height;
    } else {
        region[0] = offset_x + (filter->region[0] - document->view_x) * scale_x;
        region[1] = offset_y + (filter->region[1] - document->view_y) * scale_y;
        region[2] = region[0] + filter->region[2] * scale_x;
        region[3] = region[1] + filter->region[3] * scale_y;
    }
    for (uint32_t y = 0U; y < tile->height; ++y) {
        double sample_y = (double)y + 0.5;
        for (uint32_t x = 0U; x < tile->width; ++x) {
            double sample_x = (double)x + 0.5;
            if (sample_x >= region[0] && sample_y >= region[1] &&
                sample_x < region[2] && sample_y < region[3]) continue;
            memset(tile->rgba + ((size_t)y * tile->width + x) * 4U, 0, 4U);
        }
    }
}

bool raster_shape_filter_apply(const svg_shape_document *document,
                               size_t filter_index, raster_tile *tile,
                               size_t available_scratch,
                               svg_shape_diagnostic *diagnostic)
{
    if (document == NULL || tile == NULL || tile->rgba == NULL ||
        document->filters == NULL || filter_index >= document->filter_count ||
        document->filter_primitives == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    const svg_shape_filter *filter = &document->filters[filter_index];
    if (filter->primitive_count == 0U ||
        filter->primitive_count > SVG_SHAPE_MAX_FILTER_PRIMITIVES ||
        filter->first_primitive > document->filter_primitive_count ||
        filter->primitive_count > document->filter_primitive_count -
                                  filter->first_primitive) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    if (tile->width == 0U || tile->height == 0U ||
        (size_t)tile->width > SIZE_MAX / (size_t)tile->height) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return false;
    }
    size_t pixels = (size_t)tile->width * tile->height;
    if (pixels > SIZE_MAX / 4U || tile->byte_count != pixels * 4U) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return false;
    }
    double source_bounds[4];
    bool have_source_bounds = false;
    find_alpha_bounds(tile->rgba, tile->width, tile->height,
                     source_bounds, &have_source_bounds);
    double scale_x = 0.0, scale_y = 0.0, offset_x = 0.0, offset_y = 0.0;
    if (!viewport_mapping(document, tile, &scale_x, &scale_y,
                          &offset_x, &offset_y)) {
        fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, 0U);
        return false;
    }
    double box_width = have_source_bounds ? source_bounds[2] - source_bounds[0] : 0.0;
    double box_height = have_source_bounds ? source_bounds[3] - source_bounds[1] : 0.0;
    svg_shape_filter_primitive scaled[SVG_SHAPE_MAX_FILTER_PRIMITIVES];
    memcpy(scaled, document->filter_primitives + filter->first_primitive,
           filter->primitive_count * sizeof(*scaled));
    for (size_t i = 0U; i < filter->primitive_count; ++i) {
        svg_shape_filter_primitive *primitive = &scaled[i];
        if (primitive->kind != SVG_SHAPE_FILTER_GAUSSIAN_BLUR &&
            primitive->kind != SVG_SHAPE_FILTER_DROP_SHADOW &&
            primitive->kind != SVG_SHAPE_FILTER_OFFSET) continue;
        if (filter->primitive_units_object_bounding_box) {
            primitive->values[0] *= box_width;
            primitive->values[1] *= box_height;
            primitive->values[2] *= box_width;
            primitive->values[3] *= box_height;
        } else {
            primitive->values[0] *= scale_x;
            primitive->values[1] *= scale_y;
            primitive->values[2] *= scale_x;
            primitive->values[3] *= scale_y;
        }
        if (!isfinite(primitive->values[0]) ||
            !isfinite(primitive->values[1]) ||
            !isfinite(primitive->values[2]) ||
            !isfinite(primitive->values[3]) ||
            primitive->values[0] > 32.0 || primitive->values[1] > 32.0 ||
            fabs(primitive->values[2]) > 4096.0 ||
            fabs(primitive->values[3]) > 4096.0) {
            fail(diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 primitive->source_offset);
            return false;
        }
    }
    size_t work = 0U;
    if (!filter_work(scaled, filter->primitive_count, pixels, &work)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT,
             document->filter_primitives[filter->first_primitive].source_offset);
        return false;
    }
    (void)work;
    filter_surfaces surfaces = {0};
    for (size_t i = 0U; i < filter->primitive_count; ++i) {
        const svg_shape_filter_primitive *primitive =
            &document->filter_primitives[filter->first_primitive + i];
        surfaces.has_alpha |= input_is_alpha(primitive);
        surfaces.has_drop_shadow |= primitive->kind == SVG_SHAPE_FILTER_DROP_SHADOW;
    }
    size_t surface_count = filter->primitive_count + 1U +
                           (surfaces.has_alpha ? 1U : 0U);
    size_t scratch = 0U;
    if (!multiply_size(surface_count, tile->byte_count, &scratch)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return false;
    }
    size_t float_count = 0U, float_bytes = 0U;
    if (!multiply_size(pixels, 4U, &float_count) ||
        !multiply_size(float_count, sizeof(float), &float_bytes) ||
        !add_size(&scratch, float_bytes) ||
        (surfaces.has_drop_shadow && !add_size(&scratch, pixels)) ||
        scratch > FILTER_SCRATCH_LIMIT || scratch > available_scratch) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT,
             document->filter_primitives[filter->first_primitive].source_offset);
        return false;
    }
    surfaces.scratch_bytes = scratch;
    if (!create_surface(tile->width, tile->height, &surfaces.graphic)) goto oom;
    memcpy(surfaces.graphic.rgba, tile->rgba, tile->byte_count);
    if (filter->linear_rgb) {
        for (size_t pixel = 0U; pixel < pixels; ++pixel)
            for (size_t c = 0U; c < 3U; ++c)
                surfaces.graphic.rgba[pixel * 4U + c] =
                    (uint8_t)floor(decode_srgb(
                        surfaces.graphic.rgba[pixel * 4U + c]) * 255.0 + 0.5);
    }
    if (surfaces.has_alpha) {
        if (!create_surface(tile->width, tile->height, &surfaces.alpha)) goto oom;
        for (size_t pixel = 0U; pixel < pixels; ++pixel)
            surfaces.alpha.rgba[pixel * 4U + 3U] =
                surfaces.graphic.rgba[pixel * 4U + 3U];
    }
    if (float_bytes != 0U) {
        surfaces.blur_work = malloc(float_bytes);
        if (surfaces.blur_work == NULL) goto oom;
    }
    if (surfaces.has_drop_shadow) {
        surfaces.shadow_alpha = malloc(pixels);
        if (surfaces.shadow_alpha == NULL) goto oom;
    }
    for (size_t i = 0U; i < filter->primitive_count; ++i) {
        if (!create_surface(tile->width, tile->height,
                            &surfaces.results[i])) goto oom;
    }
    for (size_t i = 0U; i < filter->primitive_count; ++i) {
        const svg_shape_filter_primitive *primitive = &scaled[i];
        const uint8_t *input = resolve_input(&surfaces, primitive, i,
                                             primitive->input);
        const uint8_t *input2 = resolve_input(&surfaces, primitive, i,
                                              primitive->input2);
        uint8_t *output = surfaces.results[i].rgba;
        if (input == NULL || output == NULL) {
            fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT,
                 primitive->source_offset);
            destroy_surfaces(&surfaces);
            return false;
        }
        if (primitive->kind == SVG_SHAPE_FILTER_GAUSSIAN_BLUR) {
            if (!blur(input, output, tile->width, tile->height,
                      primitive->values[0], primitive->values[1],
                      surfaces.blur_work)) goto invalid;
        } else if (primitive->kind == SVG_SHAPE_FILTER_OFFSET) {
            offset_surface(input, output, tile->width, tile->height,
                           primitive->values[2], primitive->values[3]);
        } else if (primitive->kind == SVG_SHAPE_FILTER_FLOOD) {
            uint8_t color[4];
            memcpy(color, primitive->rgba, sizeof(color));
            if (filter->linear_rgb)
                for (size_t c = 0U; c < 3U; ++c)
                    color[c] = (uint8_t)floor(decode_srgb(color[c]) * 255.0 + 0.5);
            for (size_t pixel = 0U; pixel < pixels; ++pixel)
                memcpy(output + pixel * 4U, color, 4U);
        } else if (primitive->kind == SVG_SHAPE_FILTER_BLEND ||
                   primitive->kind == SVG_SHAPE_FILTER_COMPOSITE) {
            if (input2 == NULL) goto invalid;
            composite(primitive, input, input2, output, pixels);
        } else if (primitive->kind == SVG_SHAPE_FILTER_MERGE) {
            for (size_t merge = 0U; merge < primitive->merge_count; ++merge) {
                const uint8_t *merge_input = resolve_input(
                    &surfaces, primitive, i, primitive->merge_inputs[merge]);
                if (merge_input == NULL) goto invalid;
                for (size_t pixel = 0U; pixel < pixels; ++pixel)
                    source_over(output + pixel * 4U,
                                merge_input + pixel * 4U);
            }
        } else if (primitive->kind == SVG_SHAPE_FILTER_COLOR_MATRIX) {
            color_matrix(primitive, input, output, pixels);
        } else if (primitive->kind == SVG_SHAPE_FILTER_DROP_SHADOW) {
            if (!drop_shadow(primitive, input, output, surfaces.shadow_alpha,
                             surfaces.blur_work, tile->width, tile->height,
                             filter->linear_rgb)) goto invalid;
        } else goto invalid;
    }
    memcpy(tile->rgba,
           surfaces.results[filter->primitive_count - 1U].rgba,
           tile->byte_count);
    if (filter->linear_rgb) {
        for (size_t pixel = 0U; pixel < pixels; ++pixel)
            for (size_t c = 0U; c < 3U; ++c)
                tile->rgba[pixel * 4U + c] =
                    encode_srgb((double)tile->rgba[pixel * 4U + c] / 255.0);
    }
    clip_filter_region(document, filter, tile, source_bounds,
                       have_source_bounds, scale_x, scale_y,
                       offset_x, offset_y);
    destroy_surfaces(&surfaces);
    return true;

invalid:
    fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
         document->filter_primitives[filter->first_primitive].source_offset);
    destroy_surfaces(&surfaces);
    return false;
oom:
    fail(diagnostic, SVG_SHAPE_ERROR_OOM,
         document->filter_primitives[filter->first_primitive].source_offset);
    destroy_surfaces(&surfaces);
    return false;
}
