# Owned SVG CSS

### Marker expansion context

`CssPaint::without_inherited_markers` returns a copied computed style clearing
only Marker/MarkerStart/MarkerMid/MarkerEnd, matching the owned legacy marker
expansion. Other properties and original-source spans remain unchanged. Parent
styles remain intact; explicit child declarations still use the normal cascade.
The four-field operation allocates nothing and provides no general mutable CSS
escape. Full definition planning/execution remains a separate acceptance gate.
