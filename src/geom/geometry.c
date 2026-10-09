#include "geometry.h"

#include <math.h>

bool geom_finite_point(geom_point point)
{
    return isfinite(point.x) && isfinite(point.y);
}

geom_affine geom_affine_identity(void)
{
    return (geom_affine){1.0, 0.0, 0.0, 1.0, 0.0, 0.0};
}

bool geom_affine_valid(geom_affine m)
{
    return isfinite(m.a) && isfinite(m.b) && isfinite(m.c) && isfinite(m.d) &&
           isfinite(m.e) && isfinite(m.f) && fabs(m.a) <= 1e12 &&
           fabs(m.b) <= 1e12 && fabs(m.c) <= 1e12 && fabs(m.d) <= 1e12 &&
           fabs(m.e) <= 1e6 && fabs(m.f) <= 1e6;
}

bool geom_affine_compose(geom_affine l, geom_affine r, geom_affine *out)
{
    if (out == NULL || !geom_affine_valid(l) || !geom_affine_valid(r)) return false;
    geom_affine m = {l.a*r.a+l.c*r.b, l.b*r.a+l.d*r.b,
                     l.a*r.c+l.c*r.d, l.b*r.c+l.d*r.d,
                     l.a*r.e+l.c*r.f+l.e, l.b*r.e+l.d*r.f+l.f};
    if (!geom_affine_valid(m)) return false;
    *out = m;
    return true;
}

bool geom_affine_map_point(geom_affine m, geom_point p, geom_point *out)
{
    if (out == NULL || !geom_affine_valid(m) || !geom_finite_point(p)) return false;
    geom_point q = {m.a*p.x+m.c*p.y+m.e, m.b*p.x+m.d*p.y+m.f};
    if (!geom_finite_point(q) || fabs(q.x) > 1e6 || fabs(q.y) > 1e6) return false;
    *out = q;
    return true;
}

bool geom_affine_map_bounds(geom_affine m, double x, double y, double w, double h,
                            double bounds[4])
{
    if (bounds == NULL || !isfinite(x) || !isfinite(y) || !isfinite(w) ||
        !isfinite(h) || w < 0.0 || h < 0.0) return false;
    geom_point p[4] = {{x,y},{x+w,y},{x+w,y+h},{x,y+h}}, q;
    double min_x=INFINITY,min_y=INFINITY,max_x=-INFINITY,max_y=-INFINITY;
    for (size_t i=0;i<4U;i++) {
        if (!geom_affine_map_point(m,p[i],&q)) return false;
        min_x=fmin(min_x,q.x); min_y=fmin(min_y,q.y);
        max_x=fmax(max_x,q.x); max_y=fmax(max_y,q.y);
    }
    bounds[0]=min_x; bounds[1]=min_y; bounds[2]=max_x-min_x; bounds[3]=max_y-min_y;
    return true;
}
