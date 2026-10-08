cmake_minimum_required(VERSION 3.25)

if(NOT CMAKE_OSX_DEPLOYMENT_TARGET)
    set(CMAKE_OSX_DEPLOYMENT_TARGET "13.0" CACHE STRING "Minimum supported macOS version" FORCE)
endif()

project(vexel VERSION 0.0.1 LANGUAGES C)

if(NOT APPLE)
    message(FATAL_ERROR "vexel currently supports macOS only")
endif()

if(NOT CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$")
    message(FATAL_ERROR "vexel currently requires Apple Silicon (arm64)")
endif()

add_executable(svgworkbench app/main.c)
set_target_properties(svgworkbench PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO
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
