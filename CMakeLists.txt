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

set(VEXEL_SANITIZER "" CACHE STRING "Sanitizer profile: asan-ubsan or ubsan")
set_property(CACHE VEXEL_SANITIZER PROPERTY STRINGS "" asan-ubsan ubsan)
if(VEXEL_SANITIZER)
    if(NOT CMAKE_C_COMPILER_ID STREQUAL "AppleClang")
        message(FATAL_ERROR "${VEXEL_SANITIZER} requires Xcode Apple Clang; found ${CMAKE_C_COMPILER_ID}")
    endif()
    if(VEXEL_SANITIZER STREQUAL "asan-ubsan")
        set(VEXEL_SANITIZER_FLAGS -fsanitize=address,undefined -fno-omit-frame-pointer)
    elseif(VEXEL_SANITIZER STREQUAL "ubsan")
        set(VEXEL_SANITIZER_FLAGS -fsanitize=undefined -fno-omit-frame-pointer)
    else()
        message(FATAL_ERROR "Unsupported VEXEL_SANITIZER='${VEXEL_SANITIZER}'; use asan-ubsan or ubsan")
    endif()
endif()

add_executable(svgworkbench app/main.c app/test_driver/native_task_tests.c src/raster/tile.c
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

add_library(vexel_base STATIC src/base/base.c)

add_library(vexel_xml STATIC
    src/svg/xml/xml_chars.c
    src/svg/xml/xml_tokenizer.c
)
target_include_directories(vexel_xml PUBLIC src/svg/xml src/base)
target_link_libraries(vexel_xml PUBLIC vexel_base)
set_target_properties(vexel_xml PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_library(vexel_document STATIC
    src/document/source_save.c
    src/platform/macos/platform_atomic_file.c
)
target_include_directories(vexel_document PUBLIC src/document src/platform src/svg/xml src/base)
target_link_libraries(vexel_document PUBLIC vexel_xml vexel_base)
set_target_properties(vexel_document PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO)

add_executable(xml_tokenizer_test tests/svg/xml/test_xml_tokenizer.c)
target_link_libraries(xml_tokenizer_test PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(xml_tokenizer_cli tests/svg/xml/xml_tokenizer_cli.c)
target_link_libraries(xml_tokenizer_cli PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_cli PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(xml_tokenizer_fuzz tests/svg/xml/xml_tokenizer_fuzz.c)
target_link_libraries(xml_tokenizer_fuzz PRIVATE vexel_xml)
set_target_properties(xml_tokenizer_fuzz PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")

add_executable(source_save_test tests/document/test_source_save.c)
target_link_libraries(source_save_test PRIVATE vexel_document)
set_target_properties(source_save_test PROPERTIES C_STANDARD 17 C_STANDARD_REQUIRED YES C_EXTENSIONS NO RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
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
    target_compile_options(vexel_document PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_cli PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(xml_tokenizer_fuzz PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(base_contract_test PRIVATE ${VEXEL_STRICT_WARNINGS})
    target_compile_options(source_save_test PRIVATE ${VEXEL_STRICT_WARNINGS})
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
    target_compile_options(vexel_document PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(vexel_document PRIVATE ${VEXEL_SANITIZER_FLAGS})
    foreach(xml_target xml_tokenizer_test xml_tokenizer_cli xml_tokenizer_fuzz source_save_test)
        target_compile_options(${xml_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
        target_link_options(${xml_target} PRIVATE ${VEXEL_SANITIZER_FLAGS})
    endforeach()
endif()

include(CTest)
if(BUILD_TESTING)
    add_test(NAME cli-version COMMAND svgworkbench --version)
    add_test(NAME base-contract COMMAND base_contract_test)
    add_test(NAME xml-tokenizer COMMAND xml_tokenizer_test)
    add_test(NAME source-save COMMAND source_save_test)
endif()
