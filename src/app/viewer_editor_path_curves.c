#include "viewer_editor_internal.h"

#include "document_model.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define PATH_COORDINATE_LIMIT 1000000.0
#define PATH_PI 3.14159265358979323846
#define PATH_TWO_PI (2.0 * PATH_PI)
#define PATH_CURVE_SAMPLE_COUNT 64U
#define PATH_SOURCE_REPLACEMENT_CAP 1024U

typedef enum path_geometry_kind {
    PATH_GEOMETRY_NONE,
    PATH_GEOMETRY_LINE,
    PATH_GEOMETRY_QUADRATIC,
    PATH_GEOMETRY_CUBIC,
    PATH_GEOMETRY_ARC
} path_geometry_kind;

typedef struct path_geometry {
    size_t segment_index;
    path_geometry_kind kind;
    geom_point start;
    geom_point end;
    geom_point control1;
    geom_point control2;
    double rx;
    double ry;
    double rotation;
    double arc_start;
    double arc_delta;
    bool arc_sweep;
} path_geometry;

typedef bool (*path_geometry_visitor)(const path_geometry *geometry,
                                      void *context);

typedef struct path_walker_state {
    geom_point current;
    geom_point subpath_start;
    geom_point cubic_control;
    geom_point quadratic_control;
    bool has_cubic_control;
    bool has_quadratic_control;
} path_walker_state;

static bool finite_point(geom_point point)
{
    return isfinite(point.x) && isfinite(point.y) &&
        fabs(point.x) <= PATH_COORDINATE_LIMIT &&
        fabs(point.y) <= PATH_COORDINATE_LIMIT;
}

static bool point_close(geom_point left, geom_point right)
{
    double scale = fmax(1.0, fmax(fmax(fabs(left.x), fabs(left.y)),
        fmax(fabs(right.x), fabs(right.y))));
    return hypot(left.x - right.x, left.y - right.y) <= 1.0e-9 * scale;
}

static bool path_number(const svg_shape_document *document,
    const svg_shape_path_segment *segment, size_t offset, double *out_value)
{
    if (document == NULL || segment == NULL || out_value == NULL ||
        offset >= segment->number_count ||
        segment->first_number > document->path_number_count ||
        segment->number_count > document->path_number_count - segment->first_number)
        return false;
    double value = document->path_numbers[segment->first_number + offset].value;
    if (!isfinite(value) || fabs(value) > PATH_COORDINATE_LIMIT) return false;
    *out_value = value;
    return true;
}

static bool path_number_pair(const svg_shape_document *document,
    const svg_shape_path_segment *segment, size_t offset, bool relative,
    geom_point origin, geom_point *out_point)
{
    double x, y;
    if (!path_number(document, segment, offset, &x) ||
        !path_number(document, segment, offset + 1U, &y)) return false;
    if (relative) { x += origin.x; y += origin.y; }
    *out_point = (geom_point){x, y};
    return finite_point(*out_point);
}

static bool arc_parameters(geom_point start, geom_point end,
    double rx, double ry, double rotation, bool large, bool sweep,
    path_geometry *geometry)
{
    rx = fabs(rx);
    ry = fabs(ry);
    if (!isfinite(rx) || !isfinite(ry) || !isfinite(rotation) ||
        rx > PATH_COORDINATE_LIMIT || ry > PATH_COORDINATE_LIMIT ||
        !finite_point(start) || !finite_point(end)) return false;
    if (rx == 0.0 || ry == 0.0 ||
        (start.x == end.x && start.y == end.y)) {
        geometry->kind = PATH_GEOMETRY_LINE;
        geometry->end = end;
        return true;
    }
    double phi = remainder(rotation, 360.0) * (PATH_PI / 180.0);
    double cp = cos(phi), sp = sin(phi);
    double dx = (start.x - end.x) * 0.5;
    double dy = (start.y - end.y) * 0.5;
    double xp = cp * dx + sp * dy;
    double yp = -sp * dx + cp * dy;
    double scale = xp * xp / (rx * rx) + yp * yp / (ry * ry);
    if (!isfinite(scale)) return false;
    if (scale > 1.0) {
        double factor = sqrt(scale);
        rx *= factor;
        ry *= factor;
    }
    double numerator = rx * rx * ry * ry - rx * rx * yp * yp -
        ry * ry * xp * xp;
    double denominator = rx * rx * yp * yp + ry * ry * xp * xp;
    if (!isfinite(numerator) || !isfinite(denominator)) return false;
    double ratio = denominator == 0.0 ? 0.0 : fmax(0.0, numerator / denominator);
    double coefficient = (large == sweep ? -1.0 : 1.0) * sqrt(ratio);
    double cxp = coefficient * (rx * yp / ry);
    double cyp = coefficient * (-ry * xp / rx);
    double cx = cp * cxp - sp * cyp + (start.x + end.x) * 0.5;
    double cy = sp * cxp + cp * cyp + (start.y + end.y) * 0.5;
    double ux = (xp - cxp) / rx;
    double uy = (yp - cyp) / ry;
    double vx = (-xp - cxp) / rx;
    double vy = (-yp - cyp) / ry;
    double angle_start = atan2(uy, ux);
    double delta = atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    if (!sweep && delta > 0.0) delta -= PATH_TWO_PI;
    if (sweep && delta < 0.0) delta += PATH_TWO_PI;
    geom_point center = {cx, cy};
    if (!finite_point(center) || !isfinite(angle_start) || !isfinite(delta))
        return false;
    geometry->kind = PATH_GEOMETRY_ARC;
    geometry->rx = rx;
    geometry->ry = ry;
    geometry->rotation = remainder(rotation, 360.0);
    geometry->control1 = center;
    geometry->arc_start = angle_start;
    geometry->arc_delta = delta;
    geometry->arc_sweep = sweep;
    geometry->end = end;
    return true;
}

static bool path_visit_geometries(const svg_shape_document *document,
    size_t shape, path_geometry_visitor visitor, void *context, bool *out_stopped)
{
    if (document == NULL || visitor == NULL || out_stopped == NULL ||
        document->path_segments == NULL || document->path_numbers == NULL ||
        shape >= document->shape_count ||
        document->path_segment_count > SVG_SHAPE_MAX_COMMANDS ||
        document->path_number_count > SVG_SHAPE_MAX_PATH_NUMBERS) return false;
    path_walker_state state = {0};
    bool stopped = false;
    for (size_t index = 0U; index < document->path_segment_count; ++index) {
        const svg_shape_path_segment *segment = &document->path_segments[index];
        if (segment->shape_index != shape) continue;
        char command = segment->command;
        char upper = command >= 'a' && command <= 'z' ?
            (char)(command - 'a' + 'A') : command;
        bool relative = command >= 'a' && command <= 'z';
        path_geometry geometry = {.segment_index = index,
            .kind = PATH_GEOMETRY_NONE, .start = state.current,
            .end = state.current};
        double values[7] = {0};
        if (segment->number_count > sizeof(values) / sizeof(values[0]))
            return false;
        for (size_t number = 0U; number < segment->number_count; ++number)
            if (!path_number(document, segment, number, &values[number]))
                return false;
        if (upper == 'M') {
            if (segment->number_count != 2U ||
                !path_number_pair(document, segment, 0U, relative,
                    state.current, &state.current)) return false;
            state.subpath_start = state.current;
            state.has_cubic_control = false;
            state.has_quadratic_control = false;
            geometry.end = state.current;
        } else if (upper == 'Z') {
            if (segment->number_count != 0U) return false;
            geometry.end = state.subpath_start;
            state.current = state.subpath_start;
            state.has_cubic_control = false;
            state.has_quadratic_control = false;
        } else if (upper == 'L' || upper == 'H' || upper == 'V') {
            if ((upper == 'L' && segment->number_count != 2U) ||
                ((upper == 'H' || upper == 'V') && segment->number_count != 1U))
                return false;
            geom_point end = state.current;
            if (upper == 'L') {
                if (!path_number_pair(document, segment, 0U, relative,
                        state.current, &end)) return false;
            } else if (upper == 'H') {
                end.x = relative ? state.current.x + values[0] : values[0];
            } else {
                end.y = relative ? state.current.y + values[0] : values[0];
            }
            if (!finite_point(end)) return false;
            geometry.kind = PATH_GEOMETRY_LINE;
            geometry.end = end;
            state.current = end;
            state.has_cubic_control = false;
            state.has_quadratic_control = false;
        } else if (upper == 'C' || upper == 'S') {
            geom_point first, second, end;
            if ((upper == 'C' && segment->number_count != 6U) ||
                (upper == 'S' && segment->number_count != 4U)) return false;
            if (upper == 'C') {
                if (!path_number_pair(document, segment, 0U, relative,
                        state.current, &first) ||
                    !path_number_pair(document, segment, 2U, relative,
                        state.current, &second) ||
                    !path_number_pair(document, segment, 4U, relative,
                        state.current, &end)) return false;
            } else {
                first = state.has_cubic_control ?
                    (geom_point){2.0 * state.current.x - state.cubic_control.x,
                        2.0 * state.current.y - state.cubic_control.y} :
                    state.current;
                if (!path_number_pair(document, segment, 0U, relative,
                        state.current, &second) ||
                    !path_number_pair(document, segment, 2U, relative,
                        state.current, &end)) return false;
            }
            if (!finite_point(first)) return false;
            geometry.kind = PATH_GEOMETRY_CUBIC;
            geometry.control1 = first;
            geometry.control2 = second;
            geometry.end = end;
            state.current = end;
            state.cubic_control = second;
            state.has_cubic_control = true;
            state.has_quadratic_control = false;
        } else if (upper == 'Q' || upper == 'T') {
            geom_point control, end;
            if ((upper == 'Q' && segment->number_count != 4U) ||
                (upper == 'T' && segment->number_count != 2U)) return false;
            if (upper == 'Q') {
                if (!path_number_pair(document, segment, 0U, relative,
                        state.current, &control) ||
                    !path_number_pair(document, segment, 2U, relative,
                        state.current, &end)) return false;
            } else {
                control = state.has_quadratic_control ?
                    (geom_point){2.0 * state.current.x -
                            state.quadratic_control.x,
                        2.0 * state.current.y - state.quadratic_control.y} :
                    state.current;
                if (!path_number_pair(document, segment, 0U, relative,
                        state.current, &end)) return false;
            }
            if (!finite_point(control)) return false;
            geometry.kind = PATH_GEOMETRY_QUADRATIC;
            geometry.control1 = control;
            geometry.end = end;
            state.current = end;
            state.quadratic_control = control;
            state.has_quadratic_control = true;
            state.has_cubic_control = false;
        } else if (upper == 'A') {
            if (segment->number_count != 7U ||
                (values[3] != 0.0 && values[3] != 1.0) ||
                (values[4] != 0.0 && values[4] != 1.0)) return false;
            geom_point end;
            if (!path_number_pair(document, segment, 5U, relative,
                    state.current, &end)) return false;
            geometry.end = end;
            if (!arc_parameters(state.current, end, values[0], values[1],
                    values[2], values[3] == 1.0, values[4] == 1.0,
                    &geometry)) return false;
            state.current = end;
            state.has_cubic_control = false;
            state.has_quadratic_control = false;
        } else return false;
        if (geometry.kind != PATH_GEOMETRY_NONE &&
            !visitor(&geometry, context)) {
            stopped = true;
            break;
        }
    }
    *out_stopped = stopped;
    return true;
}

static bool geometry_evaluate(const path_geometry *geometry, double t,
    geom_affine transform, geom_point *out_point)
{
    if (geometry == NULL || out_point == NULL || !isfinite(t) ||
        t < 0.0 || t > 1.0) return false;
    double u = 1.0 - t;
    geom_point point;
    if (geometry->kind == PATH_GEOMETRY_LINE) {
        point = (geom_point){u * geometry->start.x + t * geometry->end.x,
            u * geometry->start.y + t * geometry->end.y};
    } else if (geometry->kind == PATH_GEOMETRY_QUADRATIC) {
        point = (geom_point){u * u * geometry->start.x +
                2.0 * u * t * geometry->control1.x +
                t * t * geometry->end.x,
            u * u * geometry->start.y + 2.0 * u * t * geometry->control1.y +
                t * t * geometry->end.y};
    } else if (geometry->kind == PATH_GEOMETRY_CUBIC) {
        point = (geom_point){u * u * u * geometry->start.x +
                3.0 * u * u * t * geometry->control1.x +
                3.0 * u * t * t * geometry->control2.x +
                t * t * t * geometry->end.x,
            u * u * u * geometry->start.y +
                3.0 * u * u * t * geometry->control1.y +
                3.0 * u * t * t * geometry->control2.y +
                t * t * t * geometry->end.y};
    } else if (geometry->kind == PATH_GEOMETRY_ARC) {
        double angle = geometry->arc_start + geometry->arc_delta * t;
        double phi = geometry->rotation * (PATH_PI / 180.0);
        double cp = cos(phi), sp = sin(phi);
        double x = geometry->rx * cos(angle);
        double y = geometry->ry * sin(angle);
        point = (geom_point){geometry->control1.x + cp * x - sp * y,
            geometry->control1.y + sp * x + cp * y};
        if (t == 1.0) point = geometry->end;
    } else return false;
    return finite_point(point) && geom_affine_map_point(transform, point, out_point) &&
        finite_point(*out_point);
}

typedef struct path_candidate {
    const svg_shape_document *document;
    geom_point user;
    double tolerance;
    double best_distance;
    size_t best_segment;
} path_candidate;

static bool candidate_visit(const path_geometry *geometry, void *context)
{
    path_candidate *candidate = context;
    const svg_shape_path_segment *segment =
        &candidate->document->path_segments[geometry->segment_index];
    if (geometry->kind == PATH_GEOMETRY_NONE || segment->edge_count == 0U ||
        segment->first_edge > candidate->document->edge_count ||
        segment->edge_count > candidate->document->edge_count - segment->first_edge)
        return true;
    for (size_t index = 0U; index < segment->edge_count; ++index) {
        geom_edge edge = candidate->document->edges[segment->first_edge + index];
        double dx = edge.b.x - edge.a.x;
        double dy = edge.b.y - edge.a.y;
        double length_squared = dx * dx + dy * dy;
        double t = length_squared > 0.0 ?
            ((candidate->user.x - edge.a.x) * dx +
             (candidate->user.y - edge.a.y) * dy) / length_squared : 0.0;
        if (t < 0.0) t = 0.0;
        if (t > 1.0) t = 1.0;
        double distance = hypot(candidate->user.x - (edge.a.x + t * dx),
            candidate->user.y - (edge.a.y + t * dy));
        if (distance < candidate->best_distance) {
            candidate->best_distance = distance;
            candidate->best_segment = geometry->segment_index;
        }
    }
    return true;
}

static bool closest_curve_parameter(const path_geometry *geometry,
    geom_affine transform, geom_point user, double *out_t)
{
    if (geometry == NULL || out_t == NULL) return false;
    double best_distance = INFINITY;
    size_t best_sample = 0U;
    for (size_t index = 0U; index <= PATH_CURVE_SAMPLE_COUNT; ++index) {
        double t = (double)index / (double)PATH_CURVE_SAMPLE_COUNT;
        geom_point point;
        if (!geometry_evaluate(geometry, t, transform, &point)) return false;
        double distance = hypot(user.x - point.x, user.y - point.y);
        if (distance < best_distance) { best_distance = distance; best_sample = index; }
    }
    double low = best_sample == 0U ? 0.0 :
        (double)(best_sample - 1U) / (double)PATH_CURVE_SAMPLE_COUNT;
    double high = best_sample == PATH_CURVE_SAMPLE_COUNT ? 1.0 :
        (double)(best_sample + 1U) / (double)PATH_CURVE_SAMPLE_COUNT;
    for (size_t iteration = 0U; iteration < 48U; ++iteration) {
        double first = low + (high - low) / 3.0;
        double second = high - (high - low) / 3.0;
        geom_point first_point, second_point;
        if (!geometry_evaluate(geometry, first, transform, &first_point) ||
            !geometry_evaluate(geometry, second, transform, &second_point)) return false;
        double first_distance = hypot(user.x - first_point.x,
            user.y - first_point.y);
        double second_distance = hypot(user.x - second_point.x,
            user.y - second_point.y);
        if (first_distance < second_distance) high = second;
        else low = first;
    }
    *out_t = (low + high) * 0.5;
    return isfinite(*out_t);
}

static bool path_range_current(viewer_editor *editor,
    const svg_shape_document *document, document_source_span span)
{
    base_bytes current = {0};
    return editor != NULL && document != NULL && span.offset <= document->source.len &&
        span.length <= document->source.len - span.offset &&
        document_edit_span_bytes(editor->edit, span, &current) &&
        (span.length == 0U || memcmp(current.ptr,
            document->source.ptr + span.offset, span.length) == 0);
}

static bool path_selection(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection *out_selection)
{
    base_bytes tag = {0};
    return editor != NULL && document != NULL && out_selection != NULL &&
        editor->draw_tool == VIEWER_EDITOR_TOOL_PEN &&
        document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0 &&
        svg_shape_element_at_offset(document, editor->selected_offset,
            out_selection);
}

static bool serialize_split(const path_geometry *geometry, double t,
    char output[PATH_SOURCE_REPLACEMENT_CAP], size_t *out_length)
{
    if (geometry == NULL || output == NULL || out_length == NULL ||
        !(t > 0.0 && t < 1.0)) return false;
    geom_point p0 = geometry->start, p3 = geometry->end;
    int length = -1;
    if (geometry->kind == PATH_GEOMETRY_LINE) {
        geom_point middle = {p0.x + t * (p3.x - p0.x),
            p0.y + t * (p3.y - p0.y)};
        double rounded_x = nearbyint(middle.x);
        double rounded_y = nearbyint(middle.y);
        if (fabs(middle.x - rounded_x) < 1.0e-7) middle.x = rounded_x;
        if (fabs(middle.y - rounded_y) < 1.0e-7) middle.y = rounded_y;
        length = snprintf(output, PATH_SOURCE_REPLACEMENT_CAP,
            "L %.17g %.17g L %.17g %.17g", middle.x, middle.y, p3.x, p3.y);
    } else if (geometry->kind == PATH_GEOMETRY_CUBIC) {
        geom_point p01 = {p0.x + t * (geometry->control1.x - p0.x),
            p0.y + t * (geometry->control1.y - p0.y)};
        geom_point p12 = {geometry->control1.x +
                t * (geometry->control2.x - geometry->control1.x),
            geometry->control1.y + t *
                (geometry->control2.y - geometry->control1.y)};
        geom_point p23 = {geometry->control2.x +
                t * (p3.x - geometry->control2.x),
            geometry->control2.y + t * (p3.y - geometry->control2.y)};
        geom_point p012 = {p01.x + t * (p12.x - p01.x),
            p01.y + t * (p12.y - p01.y)};
        geom_point p123 = {p12.x + t * (p23.x - p12.x),
            p12.y + t * (p23.y - p12.y)};
        geom_point middle = {p012.x + t * (p123.x - p012.x),
            p012.y + t * (p123.y - p012.y)};
        if (!finite_point(p01) || !finite_point(p012) || !finite_point(p123) ||
            !finite_point(p23) || !finite_point(middle)) return false;
        length = snprintf(output, PATH_SOURCE_REPLACEMENT_CAP,
            "C %.17g %.17g %.17g %.17g %.17g %.17g "
            "C %.17g %.17g %.17g %.17g %.17g %.17g",
            p01.x, p01.y, p012.x, p012.y, middle.x, middle.y,
            p123.x, p123.y, p23.x, p23.y, p3.x, p3.y);
    } else if (geometry->kind == PATH_GEOMETRY_QUADRATIC) {
        geom_point first = {p0.x + t * (geometry->control1.x - p0.x),
            p0.y + t * (geometry->control1.y - p0.y)};
        geom_point second = {geometry->control1.x +
                t * (p3.x - geometry->control1.x),
            geometry->control1.y + t * (p3.y - geometry->control1.y)};
        geom_point middle = {first.x + t * (second.x - first.x),
            first.y + t * (second.y - first.y)};
        if (!finite_point(first) || !finite_point(second) ||
            !finite_point(middle)) return false;
        length = snprintf(output, PATH_SOURCE_REPLACEMENT_CAP,
            "Q %.17g %.17g %.17g %.17g Q %.17g %.17g %.17g %.17g",
            first.x, first.y, middle.x, middle.y,
            second.x, second.y, p3.x, p3.y);
    } else if (geometry->kind == PATH_GEOMETRY_ARC) {
        geom_point middle;
        double first_delta = geometry->arc_delta * t;
        double second_delta = geometry->arc_delta - first_delta;
        if (!geometry_evaluate(geometry, t, (geom_affine){1, 0, 0, 1, 0, 0},
                &middle)) return false;
        unsigned first_large = fabs(first_delta) > PATH_PI ? 1U : 0U;
        unsigned second_large = fabs(second_delta) > PATH_PI ? 1U : 0U;
        unsigned sweep = geometry->arc_sweep ? 1U : 0U;
        length = snprintf(output, PATH_SOURCE_REPLACEMENT_CAP,
            "A %.17g %.17g %.17g %u %u %.17g %.17g "
            "A %.17g %.17g %.17g %u %u %.17g %.17g",
            geometry->rx, geometry->ry, geometry->rotation, first_large, sweep,
            middle.x, middle.y, geometry->rx, geometry->ry,
            geometry->rotation, second_large, sweep, p3.x, p3.y);
    }
    if (length <= 0 || (size_t)length >= PATH_SOURCE_REPLACEMENT_CAP) return false;
    *out_length = (size_t)length;
    return true;
}

static uint32_t commit_path_span(viewer_editor *editor,
    const svg_shape_document *document, document_source_span span,
    const uint8_t *replacement, size_t replacement_length, const char *status)
{
    if (!path_range_current(editor, document, span)) {
        editor_status_ok(editor, "Path source is stale; wait for preview");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_replace_source_span(editor->edit,
        span, (base_bytes){replacement, replacement_length});
    editor->path_selected_node_segment = SIZE_MAX;
    uint32_t result = editor_apply_result(editor, error, old_offset, old_id,
        (document_handle){0U});
    if ((result & VIEWER_EDITOR_MUTATED) != 0U)
        editor_status_ok(editor, status);
    return result;
}

typedef struct path_find_geometry {
    size_t wanted[2];
    size_t wanted_count;
    path_geometry values[2];
    size_t found_count;
} path_find_geometry;

static bool find_geometry_visit(const path_geometry *geometry, void *context)
{
    path_find_geometry *find = context;
    for (size_t index = 0U; index < find->wanted_count; ++index) {
        if (find->wanted[index] == geometry->segment_index) {
            find->values[index] = *geometry;
            ++find->found_count;
            break;
        }
    }
    return find->found_count < find->wanted_count;
}

static bool find_segment_geometry(const svg_shape_document *document,
    size_t shape, size_t segment, path_geometry *out)
{
    path_find_geometry find = {.wanted = {segment, 0U}, .wanted_count = 1U};
    bool stopped = false;
    if (!path_visit_geometries(document, shape, find_geometry_visit,
            &find, &stopped) || find.found_count != 1U) return false;
    (void)stopped;
    *out = find.values[0];
    return true;
}

static bool find_adjacent_geometries(const svg_shape_document *document,
    size_t shape, size_t left_segment, size_t right_segment,
    path_geometry *left, path_geometry *right)
{
    path_find_geometry find = {
        .wanted = {left_segment, right_segment}, .wanted_count = 2U};
    bool stopped = false;
    if (!path_visit_geometries(document, shape, find_geometry_visit,
            &find, &stopped) || find.found_count != 2U) return false;
    (void)stopped;
    *left = find.values[0];
    *right = find.values[1];
    return true;
}

static geom_point interpolate(geom_point start, geom_point end, double t)
{
    return (geom_point){start.x + t * (end.x - start.x),
        start.y + t * (end.y - start.y)};
}

static bool split_inverse_parameter(geom_point first, geom_point middle,
    geom_point last, double *out_t)
{
    double dx = last.x - first.x;
    double dy = last.y - first.y;
    double length_squared = dx * dx + dy * dy;
    if (!(length_squared > 1.0e-24) || !isfinite(length_squared)) return false;
    double t = ((middle.x - first.x) * dx + (middle.y - first.y) * dy) /
        length_squared;
    if (!isfinite(t) || t < 0.0001 || t > 0.9999 ||
        !point_close(interpolate(first, last, t), middle)) return false;
    *out_t = t;
    return true;
}

static bool merge_cubic(const path_geometry *left, const path_geometry *right,
    geom_point *out_control1, geom_point *out_control2)
{
    if (left->kind != PATH_GEOMETRY_CUBIC ||
        right->kind != PATH_GEOMETRY_CUBIC ||
        !point_close(left->end, right->start)) return false;
    double t;
    if (!split_inverse_parameter(left->control2, left->end,
            right->control1, &t)) return false;
    geom_point control1 = {
        (left->control1.x - (1.0 - t) * left->start.x) / t,
        (left->control1.y - (1.0 - t) * left->start.y) / t};
    geom_point control2 = {
        (right->control2.x - t * right->end.x) / (1.0 - t),
        (right->control2.y - t * right->end.y) / (1.0 - t)};
    geom_point p01 = interpolate(left->start, control1, t);
    geom_point p12 = interpolate(control1, control2, t);
    geom_point p23 = interpolate(control2, right->end, t);
    geom_point p012 = interpolate(p01, p12, t);
    geom_point p123 = interpolate(p12, p23, t);
    geom_point middle = interpolate(p012, p123, t);
    if (!finite_point(control1) || !finite_point(control2) ||
        !point_close(p01, left->control1) ||
        !point_close(p012, left->control2) ||
        !point_close(p123, right->control1) ||
        !point_close(p23, right->control2) ||
        !point_close(middle, left->end)) return false;
    *out_control1 = control1;
    *out_control2 = control2;
    return true;
}

static bool merge_quadratic(const path_geometry *left,
    const path_geometry *right, geom_point *out_control)
{
    if (left->kind != PATH_GEOMETRY_QUADRATIC ||
        right->kind != PATH_GEOMETRY_QUADRATIC ||
        !point_close(left->end, right->start)) return false;
    double t;
    if (!split_inverse_parameter(left->control1, left->end,
            right->control1, &t)) return false;
    geom_point control = {
        (left->control1.x - (1.0 - t) * left->start.x) / t,
        (left->control1.y - (1.0 - t) * left->start.y) / t};
    geom_point right_control = interpolate(control, right->end, t);
    if (!finite_point(control) || !point_close(right_control, right->control1))
        return false;
    *out_control = control;
    return true;
}

static bool merge_arc(const path_geometry *left, const path_geometry *right)
{
    if (left->kind != PATH_GEOMETRY_ARC || right->kind != PATH_GEOMETRY_ARC ||
        left->arc_sweep != right->arc_sweep ||
        !point_close(left->control1, right->control1) ||
        !point_close(left->end, right->start) ||
        !point_close((geom_point){left->rx, left->ry},
            (geom_point){right->rx, right->ry}) ||
        fabs(left->rotation - right->rotation) > 1.0e-7) return false;
    return true;
}

static bool serialize_merged_arc(const path_geometry *left,
    const path_geometry *right, char output[PATH_SOURCE_REPLACEMENT_CAP],
    size_t *out_length)
{
    double delta = left->arc_delta + right->arc_delta;
    unsigned large = fabs(delta) > PATH_PI ? 1U : 0U;
    unsigned sweep = delta >= 0.0 ? 1U : 0U;
    int length = snprintf(output, PATH_SOURCE_REPLACEMENT_CAP,
        "A %.17g %.17g %.17g %u %u %.17g %.17g", left->rx, left->ry,
        left->rotation, large, sweep, right->end.x, right->end.y);
    if (length <= 0 || (size_t)length >= PATH_SOURCE_REPLACEMENT_CAP) return false;
    *out_length = (size_t)length;
    return true;
}

static bool next_path_segment(const svg_shape_document *document,
    size_t shape, size_t selected, size_t *out_next)
{
    if (document == NULL || out_next == NULL ||
        selected >= document->path_segment_count) return false;
    for (size_t index = selected + 1U; index < document->path_segment_count; ++index) {
        if (document->path_segments[index].shape_index == shape) {
            *out_next = index;
            return true;
        }
        if (document->path_segments[index].shape_index > shape) break;
    }
    return false;
}

uint32_t editor_canvas_path_curve_delete_node(viewer_editor *editor,
    const svg_shape_document *document)
{
    svg_shape_selection selection;
    if (editor == NULL || document == NULL ||
        editor->path_selected_node_segment >= document->path_segment_count ||
        !path_selection(editor, document, &selection)) return VIEWER_EDITOR_HANDLED;
    size_t selected = editor->path_selected_node_segment;
    const svg_shape_path_segment *left_segment = &document->path_segments[selected];
    if (left_segment->command == 'M' || left_segment->command == 'm' ||
        left_segment->command == 'Z' || left_segment->command == 'z' ||
        left_segment->source_length == 0U) {
        editor_status_ok(editor, "The first path anchor cannot be deleted");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t next = SIZE_MAX;
    if (next_path_segment(document, selection.shape_index, selected, &next)) {
        const svg_shape_path_segment *right_segment = &document->path_segments[next];
        path_geometry left, right;
        if (!find_adjacent_geometries(document, selection.shape_index,
                selected, next, &left, &right)) {
            editor_status_ok(editor, "Path curve data is unavailable");
            return VIEWER_EDITOR_HANDLED;
        }
        char replacement[PATH_SOURCE_REPLACEMENT_CAP];
        size_t replacement_length = 0U;
        bool merged = false;
        if (left.kind == PATH_GEOMETRY_CUBIC &&
            merge_cubic(&left, &right, &left.control1, &left.control2)) {
            int length = snprintf(replacement, sizeof(replacement),
                "C %.17g %.17g %.17g %.17g %.17g %.17g",
                left.control1.x, left.control1.y, left.control2.x,
                left.control2.y, right.end.x, right.end.y);
            merged = length > 0 && (size_t)length < sizeof(replacement);
            replacement_length = merged ? (size_t)length : 0U;
        } else if (left.kind == PATH_GEOMETRY_QUADRATIC) {
            geom_point control;
            if (merge_quadratic(&left, &right, &control)) {
                int length = snprintf(replacement, sizeof(replacement),
                    "Q %.17g %.17g %.17g %.17g", control.x, control.y,
                    right.end.x, right.end.y);
                merged = length > 0 && (size_t)length < sizeof(replacement);
                replacement_length = merged ? (size_t)length : 0U;
            }
        } else if (merge_arc(&left, &right)) {
            merged = serialize_merged_arc(&left, &right, replacement,
                &replacement_length);
        }
        if (!merged) {
            editor_status_ok(editor,
                "This curve node cannot be removed without changing its shape");
            return VIEWER_EDITOR_HANDLED;
        }
        size_t right_end;
        if (!base_size_add(right_segment->source_offset,
                right_segment->source_length, &right_end) ||
            right_end < left_segment->source_offset) {
            editor_status_ok(editor, "Path curve source spans are invalid");
            return VIEWER_EDITOR_HANDLED;
        }
        return commit_path_span(editor, document,
            (document_source_span){left_segment->source_offset,
                right_end - left_segment->source_offset},
            (const uint8_t *)replacement, replacement_length,
            "Path node removed · shape preserved · preview updating");
    }
    return commit_path_span(editor, document,
        (document_source_span){left_segment->source_offset,
            left_segment->source_length}, NULL, 0U,
        "Path node deleted · preview updating");
}

uint32_t editor_canvas_path_curve_add_node(viewer_editor *editor,
    const svg_shape_document *document, geom_point user, double tolerance)
{
    svg_shape_selection selection;
    if (!path_selection(editor, document, &selection) || !finite_point(user) ||
        !isfinite(tolerance) || !(tolerance > 0.0) ||
        tolerance > PATH_COORDINATE_LIMIT) return 0U;
    path_candidate candidate = {.document = document, .user = user,
        .tolerance = tolerance, .best_distance = tolerance,
        .best_segment = SIZE_MAX};
    bool stopped = false;
    if (!path_visit_geometries(document, selection.shape_index,
            candidate_visit, &candidate, &stopped)) return 0U;
    (void)stopped;
    if (candidate.best_segment == SIZE_MAX) return 0U;
    path_geometry geometry;
    if (!find_segment_geometry(document, selection.shape_index,
            candidate.best_segment, &geometry)) return 0U;
    double t;
    if (!closest_curve_parameter(&geometry, selection.transform, user, &t) ||
        !(t > 0.05 && t < 0.95)) return 0U;
    const svg_shape_path_segment *segment =
        &document->path_segments[candidate.best_segment];
    document_source_span span = {segment->source_offset, segment->source_length};
    if (span.length == 0U) return 0U;
    char replacement[PATH_SOURCE_REPLACEMENT_CAP];
    size_t replacement_length = 0U;
    if (!serialize_split(&geometry, t, replacement, &replacement_length)) {
        editor_status_ok(editor, "Path segment cannot be split within the editor limits");
        return VIEWER_EDITOR_HANDLED;
    }
    return commit_path_span(editor, document, span,
        (const uint8_t *)replacement, replacement_length,
        "Path node added · shape preserved · preview updating");
}
