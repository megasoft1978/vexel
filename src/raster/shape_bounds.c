#include "shape_bounds.h"
#include "../svg/clip/clip.h"

#include <float.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_BOUNDARY_WORK_LIMIT 200000000U

typedef struct geometry_bounds {
    double left;
    double top;
    double right;
    double bottom;
    bool present;
    bool invalid;
} geometry_bounds;

static bool checked_add_size(size_t left, size_t right, size_t *out)
{
    if (right > SIZE_MAX - left) return false;
    *out = left + right;
    return true;
}

static bool checked_mul_size(size_t left, size_t right, size_t *out)
{
    if (left != 0U && right > SIZE_MAX / left) return false;
    *out = left * right;
    return true;
}

static void bounds_include(geometry_bounds *bounds, geom_point point)
{
    if (!isfinite(point.x) || !isfinite(point.y)) {
        bounds->invalid = true;
        return;
    }
    if (!bounds->present) {
        bounds->left = bounds->right = point.x;
        bounds->top = bounds->bottom = point.y;
        bounds->present = true;
        return;
    }
    bounds->left = fmin(bounds->left, point.x);
    bounds->right = fmax(bounds->right, point.x);
    bounds->top = fmin(bounds->top, point.y);
    bounds->bottom = fmax(bounds->bottom, point.y);
}

static bool pixel_range(double low, double high, uint32_t extent,
                        uint32_t *first, uint32_t *last)
{
    if (extent == 0U || !isfinite(low) || !isfinite(high) || high < low) return false;
    /* Include every 4x4 sample footprint plus one pixel for roundoff margin. */
    double first_value = ceil(low - 0.875) - 1.0;
    double last_value = floor(high - 0.125) + 1.0;
    if (last_value < 0.0 || first_value >= (double)extent) return false;
    if (first_value <= 0.0) *first = 0U;
    else *first = (uint32_t)first_value;
    if (last_value >= (double)extent) *last = extent - 1U;
    else *last = (uint32_t)last_value;
    return *first <= *last;
}

static void full_tile_bounds(const svg_shape_document *doc,
                             raster_shape_bounds *bounds)
{
    bounds->device_left = 0.0;
    bounds->device_top = 0.0;
    bounds->device_right = (double)doc->width;
    bounds->device_bottom = (double)doc->height;
    bounds->pixels = (raster_pixel_bounds){0U, doc->width - 1U,
                                           0U, doc->height - 1U, true};
}

static bool map_user_box(const svg_shape_document *doc, double scale,
                         double offset_x, double offset_y,
                         geometry_bounds user, raster_shape_bounds *out)
{
    if (!user.present && !user.invalid) {
        out->pixels = (raster_pixel_bounds){0U, 0U, 0U, 0U, false};
        return true;
    }
    if (user.invalid) {
        full_tile_bounds(doc, out);
        return true;
    }
    double left = offset_x + (user.left - doc->view_x) * scale;
    double right = offset_x + (user.right - doc->view_x) * scale;
    double top = offset_y + (user.top - doc->view_y) * scale;
    double bottom = offset_y + (user.bottom - doc->view_y) * scale;
    if (!isfinite(left) || !isfinite(right) || !isfinite(top) || !isfinite(bottom)) {
        full_tile_bounds(doc, out);
        return true;
    }
    double margin_x = fmax(1.0, fmax(fabs(left), fabs(right))) * DBL_EPSILON * 8.0;
    double margin_y = fmax(1.0, fmax(fabs(top), fabs(bottom))) * DBL_EPSILON * 8.0;
    left -= margin_x;
    right += margin_x;
    top -= margin_y;
    bottom += margin_y;
    out->device_left = left;
    out->device_top = top;
    out->device_right = right;
    out->device_bottom = bottom;
    out->pixels.visible = pixel_range(left, right, doc->width,
                                      &out->pixels.x_first, &out->pixels.x_last) &&
                          pixel_range(top, bottom, doc->height,
                                      &out->pixels.y_first, &out->pixels.y_last);
    return true;
}

static bool stroke_padding(const svg_shape_document *doc, size_t shape,
                           double *pad_x, double *pad_y)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    *pad_x = 0.0;
    *pad_y = 0.0;
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0) return true;
    geom_affine inverse = doc->shape_stroke_inverse[shape];
    double determinant = inverse.a * inverse.d - inverse.b * inverse.c;
    double forward_a = inverse.d / determinant;
    double forward_b = -inverse.c / determinant;
    double forward_c = -inverse.b / determinant;
    double forward_d = inverse.a / determinant;
    double radius = stroke->width * 0.5;
    double join_factor = stroke->join == SVG_SHAPE_JOIN_MITER ? stroke->miter_limit : 1.0;
    double cap_factor = !doc->shape_stroke_closed[shape] &&
                        stroke->cap == SVG_SHAPE_CAP_SQUARE ? sqrt(2.0) : 1.0;
    double extent = radius * fmax(join_factor, cap_factor);
    *pad_x = extent * hypot(forward_a, forward_b);
    *pad_y = extent * hypot(forward_c, forward_d);
    return isfinite(*pad_x) && isfinite(*pad_y);
}

bool raster_shape_bounds_build(const svg_shape_document *doc, double scale,
                               double offset_x, double offset_y,
                               raster_shape_bounds_set *out,
                               svg_shape_diagnostic *diagnostic)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
    if (out == NULL) return false;
    *out = (raster_shape_bounds_set){0};
    if (doc == NULL || doc->width == 0U || doc->height == 0U ||
        doc->shape_count > SIZE_MAX / sizeof(*out->shapes)) return false;
    if (doc->shape_count == 0U) return true;
    geometry_bounds *fills = calloc(doc->shape_count, sizeof(*fills));
    geometry_bounds *strokes = calloc(doc->shape_count, sizeof(*strokes));
    raster_shape_bounds *shapes = calloc(doc->shape_count, sizeof(*shapes));
    if (fills == NULL || strokes == NULL || shapes == NULL) {
        free(fills); free(strokes); free(shapes);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    for (size_t i = 0U; i < doc->edge_count; i++) {
        size_t shape = doc->shape_ids[i];
        if (doc->shape_filled[shape]) {
            bounds_include(&fills[shape], doc->edges[i].a);
            bounds_include(&fills[shape], doc->edges[i].b);
        }
    }
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        size_t first = doc->shape_edge_start[shape];
        size_t count = doc->shape_edge_count[shape];
        bool stroked = doc->strokes[shape].rgba[3] != 0U &&
                       doc->strokes[shape].width > 0.0 && count != 0U;
        if (stroked) {
            double pad_x, pad_y;
            for (size_t edge = 0U; edge < count; edge++) {
                bounds_include(&strokes[shape], doc->edges[first + edge].a);
                bounds_include(&strokes[shape], doc->edges[first + edge].b);
            }
            if (!stroke_padding(doc, shape, &pad_x, &pad_y)) {
                strokes[shape].invalid = true;
            } else if (strokes[shape].present && !strokes[shape].invalid) {
                strokes[shape].left -= pad_x;
                strokes[shape].right += pad_x;
                strokes[shape].top -= pad_y;
                strokes[shape].bottom += pad_y;
            }
        }
        geometry_bounds combined = fills[shape];
        if (strokes[shape].present || strokes[shape].invalid) {
            if (!combined.present) combined = strokes[shape];
            else {
                combined.left = fmin(combined.left, strokes[shape].left);
                combined.right = fmax(combined.right, strokes[shape].right);
                combined.top = fmin(combined.top, strokes[shape].top);
                combined.bottom = fmax(combined.bottom, strokes[shape].bottom);
                combined.invalid |= strokes[shape].invalid;
            }
        }
        if (!map_user_box(doc, scale, offset_x, offset_y, combined, &shapes[shape])) {
            free(fills); free(strokes); free(shapes);
            return false;
        }
        if (shapes[shape].pixels.visible) {
            size_t width = (size_t)shapes[shape].pixels.x_last -
                           shapes[shape].pixels.x_first + 1U;
            size_t height = (size_t)shapes[shape].pixels.y_last -
                            shapes[shape].pixels.y_first + 1U;
            size_t area;
            if (!checked_mul_size(width, height, &area) ||
                !checked_add_size(out->pixel_count, area, &out->pixel_count)) {
                free(fills); free(strokes); free(shapes);
                return false;
            }
        }
    }
    free(fills);
    free(strokes);
    out->shapes = shapes;
    out->shape_count = doc->shape_count;
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    return true;
}

void raster_shape_bounds_destroy(raster_shape_bounds_set *bounds)
{
    if (bounds == NULL) return;
    free(bounds->shapes);
    *bounds = (raster_shape_bounds_set){0};
}

static bool edge_stroke_radius(const svg_shape_document *doc, size_t shape,
                               double *radius)
{
    *radius = 0.0;
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0) return true;
    geom_affine inverse = doc->shape_stroke_inverse[shape];
    double determinant = inverse.a * inverse.d - inverse.b * inverse.c;
    double a = inverse.d / determinant;
    double b = -inverse.b / determinant;
    double c = -inverse.c / determinant;
    double d = inverse.a / determinant;
    double scale_bound = sqrt(a*a + b*b + c*c + d*d);
    double join_bound = stroke->join == SVG_SHAPE_JOIN_MITER ? stroke->miter_limit : 1.0;
    double cap_bound = !doc->shape_stroke_closed[shape] &&
                       stroke->cap == SVG_SHAPE_CAP_SQUARE ? sqrt(2.0) : 1.0;
    *radius = stroke->width * 0.5 * scale_bound * fmax(join_bound, cap_bound);
    return isfinite(*radius);
}

static bool boundary_pixel_range(double low, double high, uint32_t extent,
                                 uint32_t *first, uint32_t *last)
{
    if (extent == 0U || !isfinite(low) || !isfinite(high) || high < low) return false;
    double first_value = ceil(low - 1.0) - 1.0;
    double last_value = floor(high) + 1.0;
    if (last_value < 0.0 || first_value >= (double)extent) return false;
    *first = first_value <= 0.0 ? 0U : (uint32_t)first_value;
    *last = last_value >= (double)extent ? extent - 1U : (uint32_t)last_value;
    return *first <= *last;
}

static void mark_pixel(raster_boundary_mask *mask, size_t index)
{
    uint8_t bit = (uint8_t)(1U << (index & 7U));
    uint8_t *slot = &mask->bits[index >> 3U];
    if ((*slot & bit) == 0U) mask->marked_pixel_count++;
    *slot |= bit;
}

bool raster_shape_bounds_mark_boundaries(const svg_shape_document *doc,
                                         double scale, double offset_x,
                                         double offset_y,
                                         raster_boundary_mask *out,
                                         svg_shape_diagnostic *diagnostic)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
    if (out == NULL) return false;
    *out = (raster_boundary_mask){0};
    if (doc == NULL || doc->width == 0U || doc->height == 0U ||
        (size_t)doc->width > SIZE_MAX / (size_t)doc->height) return false;
    size_t pixels = (size_t)doc->width * (size_t)doc->height;
    if (pixels > SIZE_MAX - 7U) return false;
    size_t byte_count = (pixels + 7U) / 8U;
    if (byte_count == 0U) return false;
    raster_pixel_bounds *edge_pixels = NULL;
    if (doc->edge_count > SIZE_MAX / sizeof(*edge_pixels)) return false;
    if (doc->edge_count != 0U) {
        edge_pixels = calloc(doc->edge_count, sizeof(*edge_pixels));
        if (edge_pixels == NULL) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
            return false;
        }
    }
    size_t mark_work = 0U;
    for (size_t i = 0U; i < doc->edge_count; i++) {
        size_t shape = doc->shape_ids[i];
        geom_edge edge = doc->edges[i];
        double radius;
        if (!edge_stroke_radius(doc, shape, &radius)) {
            edge_pixels[i] = (raster_pixel_bounds){0U, doc->width - 1U,
                                                   0U, doc->height - 1U, true};
        } else {
            double x1 = offset_x + (edge.a.x - doc->view_x) * scale;
            double x2 = offset_x + (edge.b.x - doc->view_x) * scale;
            double y1 = offset_y + (edge.a.y - doc->view_y) * scale;
            double y2 = offset_y + (edge.b.y - doc->view_y) * scale;
            double padding = radius * scale;
            double epsilon = fmax(1.0, fmax(fabs(x1), fmax(fabs(x2),
                                  fmax(fabs(y1), fabs(y2))))) * DBL_EPSILON * 8.0;
            uint32_t x_first, x_last, y_first, y_last;
            bool valid = isfinite(x1) && isfinite(x2) && isfinite(y1) &&
                         isfinite(y2) && isfinite(padding) && isfinite(epsilon);
            bool x_visible = valid && boundary_pixel_range(fmin(x1, x2)-padding-epsilon,
                fmax(x1, x2)+padding+epsilon, doc->width, &x_first, &x_last);
            bool y_visible = valid && boundary_pixel_range(fmin(y1, y2)-padding-epsilon,
                fmax(y1, y2)+padding+epsilon, doc->height, &y_first, &y_last);
            if (!valid) {
                edge_pixels[i] = (raster_pixel_bounds){0U, doc->width - 1U,
                                                       0U, doc->height - 1U, true};
            } else if (x_visible && y_visible) {
                edge_pixels[i] = (raster_pixel_bounds){x_first, x_last, y_first,
                                                       y_last, true};
            }
        }
        if (edge_pixels[i].visible) {
            size_t width = (size_t)edge_pixels[i].x_last - edge_pixels[i].x_first + 1U;
            size_t height = (size_t)edge_pixels[i].y_last - edge_pixels[i].y_first + 1U;
            size_t area;
            if (!checked_mul_size(width, height, &area) ||
                !checked_add_size(mark_work, area, &mark_work) ||
                mark_work > RASTER_BOUNDARY_WORK_LIMIT) {
                free(edge_pixels);
                return false;
            }
        }
    }
    uint8_t *bits = calloc(byte_count, 1U);
    if (bits == NULL) {
        free(edge_pixels);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    out->bits = bits;
    out->bit_count = pixels;
    for (size_t i = 0U; i < doc->edge_count; i++) {
        raster_pixel_bounds box = edge_pixels[i];
        if (!box.visible) continue;
        for (uint32_t y = box.y_first; y <= box.y_last; y++) {
            size_t row = (size_t)y * doc->width;
            for (uint32_t x = box.x_first; x <= box.x_last; x++)
                mark_pixel(out, row + x);
        }
    }
    out->mark_work = mark_work;
    free(edge_pixels);
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    return true;
}

typedef struct clip_boundary_context {
    const svg_shape_document *document;
    raster_boundary_mask *boundary;
    double scale;
    double offset_x;
    double offset_y;
    size_t work;
    bool force_full_mask;
    bool mark;
} clip_boundary_context;

static bool clip_edge_pixel_bounds(const clip_boundary_context *context,
                                   geom_edge edge,
                                   raster_pixel_bounds *out)
{
    const svg_shape_document *doc = context->document;
    double x1 = context->offset_x + (edge.a.x - doc->view_x) * context->scale;
    double x2 = context->offset_x + (edge.b.x - doc->view_x) * context->scale;
    double y1 = context->offset_y + (edge.a.y - doc->view_y) * context->scale;
    double y2 = context->offset_y + (edge.b.y - doc->view_y) * context->scale;
    if (!isfinite(x1) || !isfinite(x2) || !isfinite(y1) || !isfinite(y2))
        return false;
    double epsilon = fmax(1.0, fmax(fabs(x1), fmax(fabs(x2),
                    fmax(fabs(y1), fabs(y2))))) * DBL_EPSILON * 8.0;
    uint32_t x_first, x_last, y_first, y_last;
    bool x_visible = boundary_pixel_range(fmin(x1, x2) - epsilon,
        fmax(x1, x2) + epsilon, doc->width, &x_first, &x_last);
    bool y_visible = boundary_pixel_range(fmin(y1, y2) - epsilon,
        fmax(y1, y2) + epsilon, doc->height, &y_first, &y_last);
    *out = x_visible && y_visible ?
        (raster_pixel_bounds){x_first, x_last, y_first, y_last, true} :
        (raster_pixel_bounds){0U, 0U, 0U, 0U, false};
    return true;
}

static bool add_clip_edge_work(clip_boundary_context *context, size_t amount)
{
    if (amount > RASTER_BOUNDARY_WORK_LIMIT - context->work) return false;
    context->work += amount;
    return true;
}

static bool visit_clip_boundary_edge(void *opaque, geom_edge edge)
{
    clip_boundary_context *context = opaque;
    raster_pixel_bounds box;
    if (!context->mark) {
        if (!clip_edge_pixel_bounds(context, edge, &box)) {
            context->force_full_mask = true;
            return add_clip_edge_work(context, 2U);
        }
        size_t amount = 2U;
        if (box.visible) {
            size_t width = (size_t)box.x_last - box.x_first + 1U;
            size_t height = (size_t)box.y_last - box.y_first + 1U;
            size_t area;
            if (!checked_mul_size(width, height, &area) ||
                !checked_add_size(amount, area, &amount)) return false;
        }
        return add_clip_edge_work(context, amount);
    }
    if (context->force_full_mask ||
        !clip_edge_pixel_bounds(context, edge, &box)) {
        context->force_full_mask = true;
        return true;
    }
    if (!box.visible) return true;
    for (uint32_t y = box.y_first; y <= box.y_last; y++) {
        size_t row = (size_t)y * context->document->width;
        for (uint32_t x = box.x_first; x <= box.x_last; x++)
            mark_pixel(context->boundary, row + x);
    }
    return true;
}

static bool mark_full_boundary_mask(clip_boundary_context *context)
{
    size_t pixels = context->boundary->bit_count;
    if (!add_clip_edge_work(context, pixels)) return false;
    for (size_t pixel = 0U; pixel < pixels; pixel++)
        mark_pixel(context->boundary, pixel);
    return true;
}

bool raster_shape_bounds_mark_clip_boundaries(
    const svg_shape_document *doc, double scale, double offset_x,
    double offset_y, raster_boundary_mask *boundary)
{
    if (doc == NULL || boundary == NULL || boundary->bits == NULL ||
        !isfinite(scale) || scale <= 0.0 ||
        !isfinite(offset_x) || !isfinite(offset_y)) return false;
    if (doc->clips == NULL) return true;
    clip_boundary_context context = {
        .document = doc,
        .boundary = boundary,
        .scale = scale,
        .offset_x = offset_x,
        .offset_y = offset_y,
        .work = boundary->mark_work,
    };
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        if (doc->clips[shape] == NULL) continue;
        bool mapping_valid = true;
        if (!svg_clip_visit_boundary_edges(doc->clips[shape],
                visit_clip_boundary_edge, &context, &mapping_valid)) return false;
        if (!mapping_valid) {
            size_t units = svg_clip_work_units(doc->clips[shape]);
            size_t fallback_work;
            if (!checked_mul_size(units, 2U, &fallback_work) ||
                !add_clip_edge_work(&context, fallback_work)) return false;
            context.force_full_mask = true;
            break;
        }
        if (context.force_full_mask) break;
    }
    if (context.force_full_mask) {
        if (!mark_full_boundary_mask(&context)) return false;
        boundary->mark_work = context.work;
        return true;
    }
    boundary->mark_work = context.work;
    context.mark = true;
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        if (doc->clips[shape] == NULL) continue;
        bool mapping_valid = true;
        if (!svg_clip_visit_boundary_edges(doc->clips[shape],
                visit_clip_boundary_edge, &context, &mapping_valid)) return false;
        if (!mapping_valid || context.force_full_mask) {
            if (!mark_full_boundary_mask(&context)) return false;
            boundary->mark_work = context.work;
            return true;
        }
    }
    boundary->mark_work = context.work;
    return true;
}

void raster_boundary_mask_destroy(raster_boundary_mask *mask)
{
    if (mask == NULL) return;
    free(mask->bits);
    *mask = (raster_boundary_mask){0};
}

bool raster_boundary_mask_test(const raster_boundary_mask *mask, size_t pixel_index)
{
    if (mask == NULL || mask->bits == NULL || pixel_index >= mask->bit_count) return true;
    return (mask->bits[pixel_index >> 3U] & (uint8_t)(1U << (pixel_index & 7U))) != 0U;
}
