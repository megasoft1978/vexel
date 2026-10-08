# 02 — Architecture

## Layers and dependency direction

```text
app / command registry
  ├─ ui_widgets → ui_core → accessibility snapshot
  ├─ editor → doc/edit/undo → svg_parse + svg_style + scene + spatial
  ├─ scene → raster → tile cache → platform.gfx → Metal texture blit
  └─ platform services: window/input/IME/clipboard/dialog/text/menu/a11y
base ← all portable modules; OS modules never leak into portable headers
```

Portable C17 modules may depend only downward. `platform` implements opaque interfaces and may call `base`; `app` composes all. Rendering consumes immutable `scene_frame` snapshots. UI does not reach into SVG parser structs. No module includes another module's private header.

## Public API sketches

Header sketches are contracts, not final ABI. Keep public types opaque except small value structs. Each API returns status; no hidden global error.

### `base/status.h`

```c
typedef enum app_status { APP_OK, APP_E_OOM, APP_E_IO, APP_E_PARSE,
  APP_E_LIMIT, APP_E_CANCELLED, APP_E_UNSUPPORTED, APP_E_STALE,
  APP_E_PLATFORM, APP_E_INTERNAL } app_status;
typedef struct app_error { app_status code; uint64_t byte_start, byte_end;
  char message[192]; } app_error;
const char *app_status_name(app_status s);
void app_error_clear(app_error *e);
```

### `base/slice.h`

```c
typedef struct bytes { const uint8_t *ptr; size_t len; } bytes;
typedef struct mut_bytes { uint8_t *ptr; size_t len; } mut_bytes;
typedef struct str8 { const char *ptr; size_t len; } str8;
bool size_add(size_t a, size_t b, size_t *out);
bool size_mul(size_t a, size_t b, size_t *out);
bool bytes_sub(bytes in, size_t off, size_t len, bytes *out);
bool str8_eq(str8 a, str8 b);
```

### `base/arena.h`

```c
typedef struct arena arena;
typedef struct arena_mark { size_t chunk, used; } arena_mark;
app_status arena_create(size_t chunk_bytes, size_t limit, arena **out);
void *arena_push(arena *a, size_t size, size_t align);
arena_mark arena_get_mark(const arena *a);
void arena_rewind(arena *a, arena_mark mark);
void arena_reset(arena *a);
void arena_destroy(arena *a);
```

### `base/str.h`

```c
typedef struct str_builder str_builder;
app_status str_builder_init(str_builder *b, arena *a, size_t limit);
app_status str_builder_append(str_builder *b, str8 s);
app_status str_builder_append_u64(str_builder *b, uint64_t v);
str8 str_builder_view(const str_builder *b);
bool utf8_validate(str8 s, size_t *bad_offset);
```

### `platform/platform.h`

```c
typedef struct platform_window platform_window;
typedef struct platform_event platform_event;
typedef struct platform_config { str8 title; int32_t width, height; bool resizable; } platform_config;
app_status platform_init(void);
app_status platform_window_create(const platform_config *, platform_window **out);
app_status platform_poll_event(platform_window *, platform_event *out);
app_status platform_present_rgba(platform_window *, bytes pixels, uint32_t w, uint32_t h, size_t stride);
void platform_window_destroy(platform_window *w);
```

### `gfx/gfx.h`

```c
typedef struct gfx_context gfx_context;
typedef struct gfx_texture gfx_texture;
typedef struct gfx_rect { float x, y, w, h; } gfx_rect;
app_status gfx_init(platform_window *, gfx_context **out);
app_status gfx_texture_resize(gfx_context *, uint32_t w, uint32_t h, gfx_texture **out);
app_status gfx_upload(gfx_context *, gfx_texture *, gfx_rect region, bytes src, size_t stride);
app_status gfx_blit(gfx_context *, gfx_texture *, gfx_rect dst, float opacity);
void gfx_begin_frame(gfx_context *);
app_status gfx_end_frame(gfx_context *);
```

### `raster/raster.h`

```c
typedef struct raster_context raster_context;
typedef struct raster_tile raster_tile;
typedef struct raster_config { uint32_t tile_px, worker_count; size_t cache_bytes; } raster_config;
app_status raster_create(const raster_config *, raster_context **out);
app_status raster_submit(raster_context *, const scene_frame *, uint64_t generation);
app_status raster_poll_tile(raster_context *, raster_tile **out);
bytes raster_tile_pixels(const raster_tile *t, uint32_t *w, uint32_t *h, size_t *stride);
void raster_cancel_before(raster_context *, uint64_t generation);
void raster_destroy(raster_context *r);
```

### `text/text.h`

```c
typedef struct text_system text_system;
typedef struct text_run { str8 utf8; str8 family; float size_px; uint32_t weight, flags; } text_run;
typedef struct shaped_run shaped_run;
app_status text_system_create(text_system **out);
app_status text_shape(text_system *, const text_run *, shaped_run **out, app_error *err);
size_t text_glyph_count(const shaped_run *r);
void text_glyph_at(const shaped_run *r, size_t i, uint32_t *glyph, float *x, float *y);
void text_run_destroy(shaped_run *r);
void text_system_destroy(text_system *t);
```

### `svg/svg_parse.h`

```c
typedef struct svg_doc svg_doc;
typedef struct svg_limits { uint64_t max_input, max_nodes, max_text, max_path_ops;
  uint32_t max_depth, max_use_depth; } svg_limits;
typedef struct svg_progress { uint64_t bytes_done, nodes_seen; bool first_paint_ready; } svg_progress;
app_status svg_open_mapped(str8 path, const svg_limits *, svg_doc **out, app_error *err);
app_status svg_index_step(svg_doc *, size_t work_budget, svg_progress *out);
app_status svg_resolve_visible(svg_doc *, gfx_rect viewport, scene_frame **out);
bytes svg_source_bytes(const svg_doc *d);
void svg_cancel(svg_doc *d);
void svg_close(svg_doc *d);
```

### `svg/svg_style.h`

```c
typedef struct style_sheet style_sheet;
typedef struct computed_style computed_style;
typedef struct style_context { uint32_t rule_limit, selector_limit; } style_context;
app_status svg_style_parse(bytes source, arena *a, style_sheet **out, app_error *err);
app_status svg_style_compute(const style_sheet *, uint32_t element_id,
  const computed_style *parent, computed_style *out, app_error *err);
uint64_t svg_style_hash(const computed_style *s);
bool svg_style_feature_supported(str8 property_or_value);
```

### `scene/scene.h`

```c
typedef struct scene_frame scene_frame;
typedef struct scene_builder scene_builder;
typedef struct scene_item { uint32_t element_id, flags; float bounds[4]; uint64_t style_hash; } scene_item;
app_status scene_builder_create(arena *, scene_builder **out);
app_status scene_add_path(scene_builder *, uint32_t id, bytes path_data,
  const float transform[6], const computed_style *style);
app_status scene_add_rect(scene_builder *, uint32_t id, float x, float y, float w, float h,
  const computed_style *style);
app_status scene_builder_finish(scene_builder *, scene_frame **out);
size_t scene_item_count(const scene_frame *);
```

### `spatial/spatial.h`

```c
typedef struct spatial_index spatial_index;
typedef struct element_handle { uint32_t slot, generation; } element_handle;
app_status spatial_build(const scene_frame *, arena *, spatial_index **out);
app_status spatial_query(const spatial_index *, gfx_rect bounds, element_handle *out,
  size_t cap, size_t *written);
app_status spatial_hit_test(const spatial_index *, float x, float y,
  element_handle *out);
```

### `document/doc.h`

```c
typedef struct document document;
typedef struct doc_snapshot doc_snapshot;
typedef struct doc_stats { uint64_t nodes, decoded_paths, source_bytes, cache_bytes; } doc_stats;
app_status doc_create(svg_doc *, document **out);
app_status doc_snapshot_acquire(document *, doc_snapshot **out);
uint64_t doc_generation(const doc_snapshot *);
app_status doc_set_attribute(document *, element_handle, str8 name, str8 value);
app_status doc_get_stats(const document *, doc_stats *out);
void doc_snapshot_release(doc_snapshot *);
void doc_destroy(document *);
```

### `editor/edit.h` and `editor/undo.h`

```c
typedef struct edit_txn edit_txn;
app_status edit_begin(document *, str8 label, edit_txn **out);
app_status edit_transform(edit_txn *, element_handle, const float matrix[6]);
app_status edit_path_node(edit_txn *, element_handle, uint32_t node, float x, float y);
app_status edit_set_paint(edit_txn *, element_handle, str8 property, str8 value);
app_status edit_commit(edit_txn *);
void edit_cancel(edit_txn *);
app_status undo_apply(document *, bool redo);
```

### `ui/ui_core.h`

```c
typedef struct ui_context ui_context;
typedef struct ui_node_id { uint64_t value; } ui_node_id;
typedef enum ui_role { UI_ROLE_WINDOW, UI_ROLE_GROUP, UI_ROLE_BUTTON, UI_ROLE_TEXT,
  UI_ROLE_LIST, UI_ROLE_ROW, UI_ROLE_TEXTBOX, UI_ROLE_SLIDER } ui_role;
app_status ui_create(ui_context **out);
void ui_begin(ui_context *, uint64_t frame, float scale, float width, float height);
void ui_semantic_node(ui_context *, ui_node_id, ui_node_id parent, ui_role,
  str8 label, str8 value, uint32_t state);
void ui_paint_rect(ui_context *, gfx_rect, uint32_t rgba, float radius);
void ui_end(ui_context *);
void ui_dispatch(ui_context *, const platform_event *);
```

### `ui/ui_widgets.h`

```c
typedef struct ui_layout ui_layout;
typedef struct ui_style_tokens { uint32_t bg, panel, text, accent, danger;
  float spacing, radius, font_px; } ui_style_tokens;
void ui_splitter(ui_context *, ui_node_id, float *ratio, bool vertical);
bool ui_button(ui_context *, ui_node_id, str8 label, bool enabled);
bool ui_checkbox(ui_context *, ui_node_id, str8 label, bool *value);
app_status ui_text_field(ui_context *, ui_node_id, str8 label, str8 *value, bool multiline);
void ui_virtual_list(ui_context *, ui_node_id, size_t count, float row_height,
  void (*row_fn)(ui_context *, size_t, void *), void *user);
```

### `app/app.h`

```c
typedef struct app app;
typedef struct app_config { uint32_t worker_count; size_t tile_cache_bytes; bool safe_mode; } app_config;
app_status app_create(const app_config *, app **out, app_error *err);
app_status app_open(app *, str8 path, app_error *err);
app_status app_run_one_frame(app *, double monotonic_seconds, app_error *err);
app_status app_save(app *, str8 path, app_error *err);
void app_request_quit(app *);
void app_destroy(app *);
```

## Data layout sketches

```c
typedef struct source_span { uint64_t begin, end; } source_span;
typedef struct svg_element_record { uint32_t parent, first_child, next_sibling;
  source_span full; uint32_t name_id, flags; } svg_element_record; /* target 32 B */
typedef struct element_handle { uint32_t slot, generation; } element_handle;
typedef struct tile_key { int32_t x, y; uint16_t scale_bucket, color_space; uint64_t scene_generation; } tile_key;
typedef struct edit_op { uint32_t kind, target; source_span old_span; uint32_t payload_off, payload_len; } edit_op;
```

## Threading, memory, errors, frame loop

UI thread owns mutable edit overlay and command routing. Parser worker owns index build; render workers consume immutable snapshots; results carry generation IDs and are discarded if stale. Jobs are cancellable; shutdown joins workers before unmapping source. No locks around pixel loops. Memory categories are separately capped: document index, decoded geometry, visible tile cache, image cache, per-frame arena, per-job scratch. Report category counters.

Errors are explicit `app_status` plus bounded diagnostic/source span. Invalid external content yields warning and preserved source. OOM/limit errors leave prior document usable. Assertions only for programmer invariants; user files never trigger `assert`.

Frame: poll events → normalize input → route/focus/commands → apply edit transaction → update semantic/layout tree → acquire immutable doc snapshot → resolve visible scene → schedule dirty tiles → composite selection/UI paint → upload changed pixels → present → collect timings. Idle event loop sleeps until input, job completion or animation/frame deadline.
