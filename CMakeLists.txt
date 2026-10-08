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

add_executable(svgworkbench app/main.c src/platform/macos/platform_window.m)
target_include_directories(svgworkbench PRIVATE src app)
set_source_files_properties(src/platform/macos/platform_window.m PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
find_library(COCOA_FRAMEWORK Cocoa REQUIRED)
target_link_libraries(svgworkbench PRIVATE ${COCOA_FRAMEWORK})
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
    target_compile_options(svgworkbench PRIVATE
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
else()
    message(FATAL_ERROR "vexel requires Clang or GCC for its strict warning profile")
endif()

if(VEXEL_SANITIZER)
    target_compile_options(svgworkbench PRIVATE ${VEXEL_SANITIZER_FLAGS})
    target_link_options(svgworkbench PRIVATE ${VEXEL_SANITIZER_FLAGS})
endif()

include(CTest)
if(BUILD_TESTING)
    add_test(NAME cli-version COMMAND svgworkbench --version)
endif()
