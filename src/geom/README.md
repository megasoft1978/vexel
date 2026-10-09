# Geometry primitives

`geom_point`, `geom_edge` and `geom_affine` are double-precision value types in
SVG user coordinates. They own no memory. Affine matrices use
`[a c e; b d f; 0 0 1]`; composition computes `left * right`, so the right
matrix maps a point first. Mapping rejects non-finite or out-of-range results.
Bounds mapping transforms all four corners and returns an axis-aligned
`x, y, width, height` array. Matrices may be singular; callers decide whether
singularity is valid for their operation.
