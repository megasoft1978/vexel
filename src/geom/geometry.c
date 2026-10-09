#include "geometry.h"

#include <math.h>

bool geom_finite_point(geom_point point)
{
    return isfinite(point.x) && isfinite(point.y);
}
