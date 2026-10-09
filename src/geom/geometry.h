#ifndef VEXEL_GEOM_GEOMETRY_H
#define VEXEL_GEOM_GEOMETRY_H

#include <stdbool.h>
#include <stddef.h>

typedef struct geom_point { double x; double y; } geom_point;
typedef struct geom_edge { geom_point a; geom_point b; } geom_edge;

bool geom_finite_point(geom_point point);

#endif
