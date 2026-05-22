cmake_minimum_required(VERSION 3.24)

project(OpenAnimCpp VERSION 0.1.0 LANGUAGES CXX)

if (POLICY CMP0167)
    cmake_policy(SET CMP0167 NEW)
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Boost 1.80 REQUIRED COMPONENTS json)

add_library(openanim
    src/types.cpp
    src/scene_ir.cpp
    src/validation.cpp
    src/hasher.cpp
    src/renderer.cpp
    src/orchestrator.cpp
    src/engine.cpp
)

target_include_directories(openanim
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(openanim PUBLIC Boost::json)

if (MSVC)
    target_compile_options(openanim PRIVATE /W4)
else()
    target_compile_options(openanim PRIVATE -Wall -Wextra -Wpedantic)
endif()

add_executable(openanim-studio apps/studio.cpp)
target_link_libraries(openanim-studio PRIVATE openanim)

enable_testing()
add_executable(openanim-tests tests/core_tests.cpp)
target_link_libraries(openanim-tests PRIVATE openanim)
add_test(NAME openanim-core COMMAND openanim-tests)
