# vpp_runtime: the engine. DOM, style, layout, paint, fonts, and later JS.
add_library(vpp_runtime STATIC
  src/binary.cpp
  src/canvas.cpp
  src/crypto.cpp
  src/css.cpp
  src/dom.cpp
  src/font.cpp
  src/html.cpp
  src/layout.cpp
  src/package.cpp
  src/paint.cpp
  src/script.cpp
  src/sha256.cpp
  src/style.cpp
  src/template.cpp
)

target_include_directories(vpp_runtime PUBLIC include)
target_include_directories(vpp_runtime PRIVATE ${lexbor_SOURCE_DIR}/source ${quickjs_SOURCE_DIR})
target_link_libraries(vpp_runtime PRIVATE stb lexbor_static qjs ed25519)
target_compile_definitions(vpp_runtime PRIVATE LEXBOR_STATIC)

if(MSVC)
  target_compile_options(vpp_runtime PRIVATE /W4)
  target_compile_definitions(vpp_runtime PRIVATE _CRT_SECURE_NO_WARNINGS)
  # stb_truetype is a single-header library; STBTT_STATIC leaves many unused
  # static functions behind, which /W4 reports as C4505.
  set_source_files_properties(src/font.cpp PROPERTIES COMPILE_OPTIONS /wd4505)
else()
  target_compile_options(vpp_runtime PRIVATE -Wall -Wextra)
endif()
