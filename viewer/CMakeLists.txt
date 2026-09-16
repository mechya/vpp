# vpp_viewer: the user-facing native window.
add_executable(vpp_viewer
  src/main.cpp
  src/shell.cpp
)

target_link_libraries(vpp_viewer PRIVATE vpp_runtime vpp_updater SDL3::SDL3)

if(MSVC)
  target_compile_options(vpp_viewer PRIVATE /W4)
else()
  target_compile_options(vpp_viewer PRIVATE -Wall -Wextra)
endif()
