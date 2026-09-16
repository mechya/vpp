# vppc: compiles application source into VPP binary resources.
# Today: JavaScript -> code.bin. Later: HTML -> dom.bin, CSS -> style.bin.
add_executable(vppc
  src/main.cpp
)

target_link_libraries(vppc PRIVATE vpp_runtime)

if(MSVC)
  target_compile_options(vppc PRIVATE /W4)
else()
  target_compile_options(vppc PRIVATE -Wall -Wextra)
endif()
