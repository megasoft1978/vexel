cmake_minimum_required(VERSION 3.25)

if(NOT CMAKE_OSX_DEPLOYMENT_TARGET)
    set(CMAKE_OSX_DEPLOYMENT_TARGET "13.0" CACHE STRING "Minimum supported macOS version" FORCE)
endif()

project(vexel VERSION 0.0.1 LANGUAGES C OBJC)

if(NOT APPLE)
    message(FATAL_ERROR "vexel currently supports macOS only")
endif()

if(NOT CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$")
    message(FATAL_ERROR "vexel currently requires Apple Silicon (arm64)")
endif()

set(VEXEL_SANITIZER "" CACHE STRING "Sanitizer profile: asan-ubsan, ubsan, or tsan")
set_property(CACHE VEXEL_SANITIZER PROPERTY STRINGS "" asan-ubsan ubsan tsan)
if(VEXEL_SANITIZER)
    if(NOT CMAKE_C_COMPILER_ID STREQUAL "AppleClang")
        message(FATAL_ERROR "${VEXEL_SANITIZER} requires Xcode Apple Clang; found ${CMAKE_C_COMPILER_ID}")
    endif()
    if(VEXEL_SANITIZER STREQUAL "asan-ubsan")
        set(VEXEL_SANITIZER_FLAGS -fsanitize=address,undefined -fno-omit-frame-pointer)
    elseif(VEXEL_SANITIZER STREQUAL "ubsan")
        set(VEXEL_SANITIZER_FLAGS -fsanitize=undefined -fno-omit-frame-pointer)
    elseif(VEXEL_SANITIZER STREQUAL "tsan")
        set(VEXEL_SANITIZER_FLAGS -fsanitize=thread -fno-omit-frame-pointer)
    else()
        message(FATAL_ERROR "Unsupported VEXEL_SANITIZER='${VEXEL_SANITIZER}'; use asan-ubsan, ubsan, or tsan")
    endif()
endif()

add_executable(svgworkbench app/main.c app/index_replay.c app/test_driver/native_task_tests.c
    src/app/frame_compose.c src/app/viewer_load.c src/app/viewer.c src/app/render_frame_cli.c
    src/raster/tile.c
    src/platform/macos/platform_task.c src/platform/macos/platform_window.m
    src/platform/macos/platform_metal.m)
target_include_directories(svgworkbench PRIVATE src app)
set_source_files_properties(src/platform/macos/platform_window.m PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
set_source_files_properties(src/platform/macos/platform_metal.m PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
find_library(COCOA_FRAMEWORK Cocoa REQUIRED)
find_library(METAL_FRAMEWORK Metal REQUIRED)
find_library(QUARTZCORE_FRAMEWORK QuartzCore REQUIRED)
find_package(Threads REQUIRED)
target_link_libraries(svgworkbench PRIVATE ${COCOA_FRAMEWORK} ${METAL_FRAMEWORK} ${QUARTZCORE_FRAMEWORK} Threads::Threads)

# lane: viewer2
target_sources(svgworkbench PRIVATE src/app/viewer_quality.c src/app/viewer_events.c
    src/app/viewer_render.c src/platform/macos/platform_menus.m)
set_source_files_properties(src/platform/macos/platform_menus.m PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
find_library(UNIFORM_TYPE_IDENTIFIERS_FRAMEWORK UniformTypeIdentifiers REQUIRED)
target_link_libraries(svgworkbench PRIVATE ${UNIFORM_TYPE_IDENTIFIERS_FRAMEWORK})
add_executable(viewer_quality_test tests/viewer/viewer_quality_test.c src/app/viewer_quality.c)
target_include_directories(viewer_quality_test PRIVATE src)
target_link_libraries(viewer_quality_test PRIVATE vexel_ui_state)
set_target_properties(viewer_quality_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_library(vexel_base STATIC src/base/base.c)

add_library(vexel_jobs STATIC src/jobs/source_snapshot.c src/jobs/job_scheduler.c
    src/platform/macos/platform_task.c)
target_include_directories(vexel_jobs PUBLIC src)
target_link_libraries(vexel_jobs PUBLIC Threads::Threads)
set_target_properties(vexel_jobs PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
add_executable(job_snapshot_test tests/jobs/test_job_snapshot.c)
target_link_libraries(job_snapshot_test PRIVATE vexel_jobs Threads::Threads)
set_target_properties(job_snapshot_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_library(vexel_xml STATIC
    src/svg/xml/xml_chars.c
    src/svg/xml/xml_tokenizer.c
    src/svg/xml/xml_reader.c
)
target_include_directories(vexel_xml PUBLIC src/svg/xml src/base)
target_link_libraries(vexel_xml PUBLIC vexel_base)
set_target_properties(vexel_xml PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_index STATIC src/svg/index/index.c src/platform/macos/platform_task.c)
target_include_directories(vexel_index PUBLIC src src/svg/index src/svg/xml src/base)
target_link_libraries(vexel_index PUBLIC vexel_xml Threads::Threads)
set_target_properties(vexel_index PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_ui_state STATIC src/ui/index_progress.c src/ui/viewer_controls.c)
target_include_directories(vexel_ui_state PUBLIC src/ui src/svg/index src src/svg/xml src/base)
target_link_libraries(vexel_ui_state PUBLIC vexel_index)
set_target_properties(vexel_ui_state PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_ui_core STATIC src/ui/ui_core.c src/ui/ui_widgets.c)
target_include_directories(vexel_ui_core PUBLIC src src/ui src/raster src/svg/shape src/svg/css src/geom src/svg/xml src/base)
target_link_libraries(vexel_ui_core PUBLIC vexel_shape)
set_target_properties(vexel_ui_core PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
add_executable(ui_demo_render app/ui_demo_render.c app/ui_demo.c)
target_include_directories(ui_demo_render PRIVATE app src/ui)
target_link_libraries(ui_demo_render PRIVATE vexel_ui_core)
set_target_properties(ui_demo_render PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(ui_interaction_test tests/ui/ui_interaction_test.c)
target_link_libraries(ui_interaction_test PRIVATE vexel_ui_core)
set_target_properties(ui_interaction_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(ui_demo_interaction_test tests/ui/ui_demo_interaction_test.c app/ui_demo.c)
target_include_directories(ui_demo_interaction_test PRIVATE app)
target_link_libraries(ui_demo_interaction_test PRIVATE vexel_ui_core)
set_target_properties(ui_demo_interaction_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(ui_focus_bench app/ui_focus_bench.c)
target_link_libraries(ui_focus_bench PRIVATE vexel_ui_core)
set_target_properties(ui_focus_bench PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(ui_core_test tests/ui/ui_core_test.c)
target_link_libraries(ui_core_test PRIVATE vexel_ui_core)
set_target_properties(ui_core_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_library(vexel_document STATIC
    src/document/document_model.c
    src/document/source_save.c
    src/platform/macos/platform_atomic_file.c
)
target_include_directories(vexel_document PUBLIC src/document src/platform src/svg/xml src/svg/index src/base)
target_link_libraries(vexel_document PUBLIC vexel_xml vexel_base)
set_target_properties(vexel_document PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_shape STATIC
    src/geom/geometry.c
    # lane: decode — bounded CSS color parser
    src/svg/css/color.c
    src/svg/css/css.c
    src/svg/reference/reference.c
    src/svg/paint/paint.c
    src/svg/clip/clip.c
    src/svg/shape/shape.c
    # lane: decode — focused geometry decoding and rounded rectangles
    src/svg/shape/shape_geometry.c
    # lane: decode — bounded nested SVG viewport mappings
    src/svg/shape/viewport.c
    # lane: decode — color and stroke style value parsing
    src/svg/shape/shape_style.c
    # lane: decode — bounded group clipping adapter
    src/svg/shape/shape_clip_apply.c
    src/svg/shape/traverse.c
    src/svg/shape/transform.c
    src/svg/shape/document.c
    src/svg/shape/shape_alloc.c
    src/raster/shape_render.c
    # lane: decode — bounded isolated opacity layers
    src/raster/shape_render_group.c
    src/raster/shape_render_sample.c
    src/raster/shape_render_stroke.c
    src/raster/shape_bounds.c
    src/raster/shape_scanline.c
    src/raster/tile.c
)
target_include_directories(vexel_shape PUBLIC src src/geom src/svg/shape src/svg/css src/svg/reference src/raster src/svg/xml src/base)
target_link_libraries(vexel_shape PUBLIC vexel_xml vexel_base)
set_target_properties(vexel_shape PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_spatial STATIC src/spatial/index.c)
target_include_directories(vexel_spatial PUBLIC src/spatial)
set_target_properties(vexel_spatial PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_tile_cache STATIC src/raster/tile_cache.c)
target_include_directories(vexel_tile_cache PUBLIC src/raster)
set_target_properties(vexel_tile_cache PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
add_library(vexel_tile_cache_test STATIC src/raster/tile_cache.c)
target_include_directories(vexel_tile_cache_test PUBLIC src/raster)
target_compile_definitions(vexel_tile_cache_test PRIVATE VEXEL_TESTING)
set_target_properties(vexel_tile_cache_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)
target_link_libraries(svgworkbench PRIVATE vexel_shape vexel_index vexel_ui_state)

add_executable(shape_render_cli tests/svg/shape/shape_render_cli.c)
target_link_libraries(shape_render_cli PRIVATE vexel_shape)
set_target_properties(shape_render_cli PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(shape_test tests/svg/shape/test_shape.c tests/svg/shape/test_reference.c
    tests/svg/shape/test_clip_work_budget.c tests/svg/shape/test_paint_edge_span.c)
target_include_directories(shape_test PRIVATE src/svg/paint)
target_link_libraries(shape_test PRIVATE vexel_shape)
set_target_properties(shape_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(shape_render_reference_cli
    tests/svg/shape/shape_render_reference_cli.c
    tests/svg/shape/shape_render_reference.c)
target_link_libraries(shape_render_reference_cli PRIVATE vexel_shape)
set_target_properties(shape_render_reference_cli PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(shape_render_stats_cli
    tests/svg/shape/shape_render_stats_cli.c
    tests/svg/shape/shape_render_reference.c)
target_link_libraries(shape_render_stats_cli PRIVATE vexel_shape)
set_target_properties(shape_render_stats_cli PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(css_test tests/svg/css/test_css.c)
target_link_libraries(css_test PRIVATE vexel_shape)
set_target_properties(css_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(shape_fuzz tests/fuzz/shape/shape_fuzz.c)
target_link_libraries(shape_fuzz PRIVATE vexel_shape)
set_target_properties(shape_fuzz PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(spatial_cache_test tests/svg/spatial/spatial_cache_test.c tests/raster/cache/cache_contract.c src/raster/tile.c)
target_link_libraries(spatial_cache_test PRIVATE vexel_spatial vexel_tile_cache_test)
target_include_directories(spatial_cache_test PRIVATE src)
target_compile_definitions(spatial_cache_test PRIVATE VEXEL_TESTING)
set_target_properties(spatial_cache_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(spatial_cache_bench app/spatial_cache_bench.c)
target_link_libraries(spatial_cache_bench PRIVATE vexel_spatial vexel_tile_cache)
set_target_properties(spatial_cache_bench PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(xml_tokenizer_test tests/svg/xml/test_xml_tokenizer.c)
target_link_libraries(xml_tokenizer_test PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(xml_tokenizer_cli tests/svg/xml/xml_tokenizer_cli.c)
target_link_libraries(xml_tokenizer_cli PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_cli PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(xml_tokenizer_fuzz tests/svg/xml/xml_tokenizer_fuzz.c)
target_link_libraries(xml_tokenizer_fuzz PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_fuzz PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(svg_index_test tests/svg/index/test_index.c)
target_link_libraries(svg_index_test PRIVATE vexel_xml vexel_ui_state)
set_target_properties(svg_index_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(viewer_navigation_test tests/ui/viewer_navigation_test.c)
target_link_libraries(viewer_navigation_test PRIVATE vexel_ui_state)
target_include_directories(viewer_navigation_test PRIVATE src)
set_target_properties(viewer_navigation_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(viewer_frame_test tests/app/viewer_frame_test.c src/app/frame_compose.c)
target_link_libraries(viewer_frame_test PRIVATE vexel_shape vexel_ui_state)
target_include_directories(viewer_frame_test PRIVATE src)
set_target_properties(viewer_frame_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(svg_token_stream_test tests/svg/index/test_token_stream.c)
target_link_libraries(svg_token_stream_test PRIVATE vexel_xml)
set_target_properties(svg_token_stream_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
add_executable(svg_index_bench app/index_bench.c)
target_link_libraries(svg_index_bench PRIVATE vexel_shape vexel_index Threads::Threads)
set_target_properties(svg_index_bench PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(source_save_test tests/document/test_source_save.c)
target_link_libraries(source_save_test PRIVATE vexel_document)
set_target_properties(source_save_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(document_model_test tests/document/test_document_model.c)
target_link_libraries(document_model_test PRIVATE vexel_document)
set_target_properties(document_model_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

# lane: editcore
target_sources(vexel_document PRIVATE
    src/document/document_edit.c
    src/document/document_edit_ops.c
    src/document/document_edit_piece_table.c
)
add_executable(document_edit_test tests/edit/test_document_edit.c)
target_link_libraries(document_edit_test PRIVATE vexel_document)
set_target_properties(document_edit_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
target_include_directories(vexel_base PUBLIC src/base)
set_target_properties(vexel_base PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO
)

add_executable(base_contract_test tests/base/test_base.c)
target_link_libraries(base_contract_test PRIVATE vexel_base)
set_target_properties(base_contract_test PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO
)
set_target_properties(svgworkbench PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO
    OBJC_STANDARD 17
    OBJC_STANDARD_REQUIRED YES
    OBJC_EXTENSIONS NO
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)

if(CMAKE_C_COMPILER_ID MATCHES "Clang|GNU")
    set(VEXEL_STRICT_WARNINGS
        -Wall
        -Wextra
        -Wpedantic
        -Wconversion
        -Wsign-conversion
        -Wshadow
        -Wstrict-prototypes
        -Wmissing-prototypes
        -Werror
    )
    target_compile_options(svgworkbench PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_base PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_xml PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_index PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_ui_state PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_ui_core PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(ui_demo_render PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(ui_core_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(ui_focus_bench PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(ui_interaction_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(ui_demo_interaction_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_document PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_shape PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_cli PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_fuzz PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(svg_index_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(viewer_navigation_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(viewer_frame_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(svg_token_stream_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(svg_index_bench PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(base_contract_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    # lane: viewer2
    target_compile_options(viewer_quality_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(source_save_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(document_model_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(document_edit_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(shape_render_cli PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(shape_render_reference_cli PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(shape_render_stats_cli PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(shape_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(css_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(shape_fuzz PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_spatial PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_tile_cache PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_tile_cache_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(spatial_cache_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(spatial_cache_bench PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(vexel_jobs PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(job_snapshot_test PRIVATE ${VEXEL_STRICT_WARNINGS})
else()
    message(FATAL_ERROR "vexel requires Clang or GCC for its strict warning profile")
endif()

if(VEXEL_SANITIZER)
    target_compile_options(svgworkbench PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(svgworkbench PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_base PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(base_contract_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(base_contract_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_xml PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_index PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_index PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_ui_state PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_ui_state PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_ui_core PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_ui_core PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_document PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_document PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(vexel_shape PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_shape PRIVATE ${VEXEL_SANITIZER_FLAGS})
    foreach(xml_target ui_demo_render ui_focus_bench ui_core_test ui_interaction_test ui_demo_interaction_test xml_tokenizer_test xml_tokenizer_cli xml_tokenizer_fuzz svg_index_test viewer_navigation_test viewer_frame_test svg_token_stream_test svg_index_bench source_save_test document_model_test document_edit_test shape_render_cli shape_render_reference_cli shape_render_stats_cli shape_test css_test shape_fuzz spatial_cache_test spatial_cache_bench vexel_spatial vexel_tile_cache vexel_tile_cache_test)
        target_compile_options(${xml_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
        target_link_options(${xml_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
    endforeach()
    # lane: viewer2
    target_compile_options(viewer_quality_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(viewer_quality_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
endif()

if(VEXEL_SANITIZER)
    target_compile_options(vexel_jobs PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_jobs PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_compile_options(job_snapshot_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(job_snapshot_test PRIVATE ${VEXEL_SANITIZER_FLAGS})
endif()

# lane: w2-bundle
include(CTest)
add_executable(render_soak_driver
    tests/e2e/render_soak_driver.c
    src/app/viewer_load.c
    src/app/frame_compose.c
)
target_include_directories(render_soak_driver PRIVATE src)
target_link_libraries(render_soak_driver PRIVATE vexel_shape vexel_ui_state)
target_compile_options(render_soak_driver PRIVATE ${VEXEL_STRICT_WARNINGS})
set_target_properties(render_soak_driver PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)
if(VEXEL_SANITIZER)
    target_compile_options(render_soak_driver PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(render_soak_driver PRIVATE ${VEXEL_SANITIZER_FLAGS})
endif()
if(BUILD_TESTING)
    add_test(NAME render-soak-driver-bounds COMMAND render_soak_driver
        --input "${CMAKE_SOURCE_DIR}/tests/svg/shape/fixtures/rect.svg" --iterations 20)
endif()

if(BUILD_TESTING)
    add_test(NAME cli-version COMMAND svgworkbench --version)
    add_test(NAME base-contract COMMAND base_contract_test)
    add_test(NAME xml-tokenizer COMMAND xml_tokenizer_test)
    add_test(NAME source-save COMMAND source_save_test)
    add_test(NAME document-model COMMAND document_model_test)
    # lane: editcore
    add_test(NAME document-edit COMMAND document_edit_test)
    add_test(NAME shape-render COMMAND shape_test)
    add_test(NAME css-cascade COMMAND css_test)
    add_test(NAME svg-index COMMAND svg_index_test)
    add_test(NAME viewer-navigation COMMAND viewer_navigation_test)
    add_test(NAME viewer-composition COMMAND viewer_frame_test)
    # lane: viewer2
    add_test(NAME viewer-quality COMMAND viewer_quality_test)
    add_test(NAME viewer-frame COMMAND ${CMAKE_COMMAND} -E env
        VEXEL_BINARY=$<TARGET_FILE:svgworkbench>
        python3 ${CMAKE_SOURCE_DIR}/tests/test_viewer_frame.py)
    add_test(NAME native-viewer-smoke COMMAND svgworkbench --native-viewer-smoke
        "${CMAKE_SOURCE_DIR}/tests/svg/shape/fixtures/rect.svg")
    add_test(NAME ui-core COMMAND ui_core_test)
    add_test(NAME ui-interaction COMMAND ui_interaction_test)
    add_test(NAME ui-demo-semantics COMMAND ui_demo_interaction_test)
    add_test(NAME ui-editor-chrome COMMAND python3 ${CMAKE_SOURCE_DIR}/tests/ui/verify_chrome.py --binary $<TARGET_FILE:ui_demo_render>)
    add_test(NAME svg-token-stream-budget COMMAND svg_token_stream_test)
    add_test(NAME index-replay COMMAND svgworkbench --index-replay
        "${CMAKE_SOURCE_DIR}/tests/svg/shape/fixtures/rect.svg")
    add_test(NAME spatial-cache COMMAND spatial_cache_test)
    add_test(NAME job-snapshot COMMAND job_snapshot_test)
endif()

# lane: platform-media
add_library(vexel_platform_media STATIC
    src/platform/media/platform_media.c
    src/platform/media/platform_text.m
    src/platform/media/platform_image.m)
target_include_directories(vexel_platform_media PUBLIC src)
find_library(COREFOUNDATION_FRAMEWORK CoreFoundation REQUIRED)
find_library(COREGRAPHICS_FRAMEWORK CoreGraphics REQUIRED)
find_library(CORETEXT_FRAMEWORK CoreText REQUIRED)
find_library(IMAGEIO_FRAMEWORK ImageIO REQUIRED)
target_link_libraries(vexel_platform_media PUBLIC
    ${COREFOUNDATION_FRAMEWORK}
    ${COREGRAPHICS_FRAMEWORK}
    ${CORETEXT_FRAMEWORK}
    ${IMAGEIO_FRAMEWORK})
set_source_files_properties(
    src/platform/media/platform_text.m
    src/platform/media/platform_image.m
    PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
set_target_properties(vexel_platform_media PROPERTIES
    C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO
    OBJC_STANDARD 17 OBJC_STANDARD_REQUIRED YES OBJC_EXTENSIONS NO)
target_compile_options(vexel_platform_media PRIVATE ${VEXEL_STRICT_WARNINGS})

add_executable(platform_media_test tests/platform_media/platform_media_test.c)
target_link_libraries(platform_media_test PRIVATE vexel_platform_media)
set_target_properties(platform_media_test PROPERTIES
    C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
target_compile_options(platform_media_test PRIVATE ${VEXEL_STRICT_WARNINGS})

add_executable(media_probe tests/platform_media/media_probe.c)
target_link_libraries(media_probe PRIVATE vexel_platform_media)
set_target_properties(media_probe PROPERTIES
    C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
target_compile_options(media_probe PRIVATE ${VEXEL_STRICT_WARNINGS})

if(VEXEL_SANITIZER)
    foreach(media_target vexel_platform_media platform_media_test media_probe)
        target_compile_options(${media_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
        target_link_options(${media_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
    endforeach()
endif()

if(BUILD_TESTING)
    set(PLATFORM_MEDIA_FIXTURE_DIR "${CMAKE_BINARY_DIR}/platform-media-fixtures")
    add_test(NAME platform-media-fixtures COMMAND python3
        "${CMAKE_SOURCE_DIR}/tests/platform_media/generate_fixtures.py"
        --output-dir "${PLATFORM_MEDIA_FIXTURE_DIR}")
    set_tests_properties(platform-media-fixtures PROPERTIES
        FIXTURES_SETUP platform-media-generated-fixtures)
    add_test(NAME platform-media COMMAND platform_media_test
        "${PLATFORM_MEDIA_FIXTURE_DIR}")
    set_tests_properties(platform-media PROPERTIES
        FIXTURES_REQUIRED platform-media-generated-fixtures)
    add_test(NAME platform-media-python COMMAND python3
        "${CMAKE_SOURCE_DIR}/tests/test_platform_media.py")
    set_tests_properties(platform-media-python PROPERTIES
        ENVIRONMENT "VEXEL_MEDIA_PROBE=$<TARGET_FILE:media_probe>"
        FIXTURES_REQUIRED platform-media-generated-fixtures)
endif()
