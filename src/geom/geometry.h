#ifndef VEXEL_GEOM_GEOMETRY_H
#define VEXEL_GEOM_GEOMETRY_H

#include <stdbool.h>
#include <stddef.h>

typedef struct geom_point { double x; double y; } geom_point;
typedef struct geom_edge { geom_point a; geom_point b; } geom_edge;
/* SVG affine matrix [a c e; b d f; 0 0 1]. */
typedef struct geom_affine { double a, b, c, d, e, f; } geom_affine;

bool geom_finite_point(geom_point point);
geom_affine geom_affine_identity(void);
bool geom_affine_valid(geom_affine matrix);
/* out maps through right first, then left (out = left * right). */
bool geom_affine_compose(geom_affine left, geom_affine right, geom_affine *out);
bool geom_affine_map_point(geom_affine matrix, geom_point point, geom_point *out);
bool geom_affine_map_bounds(geom_affine matrix, double x, double y, double width,
                            double height, double out_bounds[4]);

#endif
