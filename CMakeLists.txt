cmake_minimum_required(VERSION 3.20)
project(vpp LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  set(CMAKE_BUILD_TYPE Release CACHE STRING "Build type" FORCE)
endif()

# ---------------------------------------------------------------------------
# Third-party dependencies, fetched at configure time. No package manager.
#
#   SDL3     window, input, presenting the pixel buffer      (zlib)
#   stb      stb_truetype glyph rasterisation                (public domain)
#   lexbor   HTML5 parser. Parsing only; the DOM, style,
#            layout and paint are VPP's own.                 (Apache 2.0)
# ---------------------------------------------------------------------------
include(FetchContent)

set(SDL_SHARED OFF CACHE BOOL "" FORCE)
set(SDL_STATIC ON CACHE BOOL "" FORCE)
set(SDL_TEST_LIBRARY OFF CACHE BOOL "" FORCE)
set(SDL_TESTS OFF CACHE BOOL "" FORCE)
set(SDL_EXAMPLES OFF CACHE BOOL "" FORCE)
FetchContent_Declare(
  SDL3
  GIT_REPOSITORY https://github.com/libsdl-org/SDL.git
  GIT_TAG        release-3.2.22
  GIT_SHALLOW    TRUE
)

FetchContent_Declare(
  stb
  GIT_REPOSITORY https://github.com/nothings/stb.git
  GIT_TAG        master
  GIT_SHALLOW    TRUE
)

set(LEXBOR_BUILD_SHARED OFF CACHE BOOL "" FORCE)
set(LEXBOR_BUILD_STATIC ON CACHE BOOL "" FORCE)
set(LEXBOR_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LEXBOR_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(LEXBOR_BUILD_UTILS OFF CACHE BOOL "" FORCE)
set(LEXBOR_INSTALL_HEADERS OFF CACHE BOOL "" FORCE)
FetchContent_Declare(
  lexbor
  GIT_REPOSITORY https://github.com/lexbor/lexbor.git
  GIT_TAG        v2.4.0
  GIT_SHALLOW    TRUE
)

# quickjs-ng: the JavaScript engine. Runs VPP bytecode; the parser is only
# needed in development builds and by the compiler.                (MIT)
set(QJS_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
set(QJS_BUILD_LIBC OFF CACHE BOOL "" FORCE)
set(QJS_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
FetchContent_Declare(
  quickjs
  GIT_REPOSITORY https://github.com/quickjs-ng/quickjs.git
  GIT_TAG        v0.16.2
  GIT_SHALLOW    TRUE
)

# ed25519 (orlp): publisher signatures. Small, portable, audited-by-use C.
# Pinned to a commit; the repository has no tags.                  (zlib)
FetchContent_Declare(
  ed25519
  GIT_REPOSITORY https://github.com/orlp/ed25519.git
  GIT_TAG        b1f19fab4aebe607805620d25a5e42566ce46a0e
)

FetchContent_MakeAvailable(SDL3 stb lexbor quickjs ed25519)

add_library(stb INTERFACE)
target_include_directories(stb INTERFACE ${stb_SOURCE_DIR})

add_library(ed25519 STATIC
  ${ed25519_SOURCE_DIR}/src/add_scalar.c
  ${ed25519_SOURCE_DIR}/src/fe.c
  ${ed25519_SOURCE_DIR}/src/ge.c
  ${ed25519_SOURCE_DIR}/src/key_exchange.c
  ${ed25519_SOURCE_DIR}/src/keypair.c
  ${ed25519_SOURCE_DIR}/src/sc.c
  ${ed25519_SOURCE_DIR}/src/seed.c
  ${ed25519_SOURCE_DIR}/src/sha512.c
  ${ed25519_SOURCE_DIR}/src/sign.c
  ${ed25519_SOURCE_DIR}/src/verify.c
)
target_include_directories(ed25519 PUBLIC ${ed25519_SOURCE_DIR}/src)
if(WIN32)
  target_link_libraries(ed25519 PUBLIC advapi32) # CryptGenRandom for seeds
endif()

# Only the qjs library is needed. The CLI tools and tests need C11 atomics,
# which MSVC 2019 lacks, so keep them out of the default build.
foreach(t qjs-libc qjsc qjs_exe run-test262 api-test lre-test function_source)
  if(TARGET ${t})
    set_target_properties(${t} PROPERTIES EXCLUDE_FROM_ALL TRUE)
  endif()
endforeach()

if(MSVC)
  # Third-party sources are UTF-8; without this MSVC warns on non-ASCII code pages.
  target_compile_options(lexbor_static PRIVATE /utf-8)
  target_compile_options(qjs PRIVATE /utf-8)
endif()

# ---------------------------------------------------------------------------
# VPP components
# ---------------------------------------------------------------------------
add_subdirectory(runtime)
add_subdirectory(compiler)
add_subdirectory(packager)
add_subdirectory(updater)
add_subdirectory(viewer)
