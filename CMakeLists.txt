cmake_minimum_required(VERSION 3.24)

project(OpenAnimCpp VERSION 0.1.0 LANGUAGES CXX)

if (POLICY CMP0167)
    cmake_policy(SET CMP0167 NEW)
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# ── Dependencies ──────────────────────────────────────────────────────────────
find_package(Boost 1.80 REQUIRED COMPONENTS json)
find_package(CURL REQUIRED)

# ── Core library ─────────────────────────────────────────────────────────────
add_library(openanim
    src/types.cpp
    src/scene_ir.cpp
    src/validation.cpp
    src/hasher.cpp
    src/sandbox.cpp
    src/renderer.cpp
    src/orchestrator.cpp
    src/llm.cpp
    src/engine.cpp
)

target_include_directories(openanim
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(openanim
    PUBLIC
        Boost::json
        CURL::libcurl
)

if (MSVC)
    target_compile_options(openanim PRIVATE /W4)
else()
    target_compile_options(openanim PRIVATE -Wall -Wextra -Wpedantic)
endif()

# ── Core unit tests ───────────────────────────────────────────────────────────
enable_testing()
add_executable(openanim-tests tests/core_tests.cpp)
target_link_libraries(openanim-tests PRIVATE openanim)
add_test(NAME openanim-core COMMAND openanim-tests)

# ── LLM E2E integration test ─────────────────────────────────────────────────
# Skipped automatically if OPENANIM_LLM_BASE_URL / API_KEY / MODEL are not set.
add_executable(openanim-tests-llm tests/test_llm_e2e.cpp)
target_link_libraries(openanim-tests-llm PRIVATE openanim)
add_test(NAME openanim-e2e-llm COMMAND openanim-tests-llm)
set_tests_properties(openanim-e2e-llm PROPERTIES LABELS "llm")
