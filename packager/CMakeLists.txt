# vpppack: builds and inspects .vpp application packages.
add_executable(vpppack
  src/main.cpp
)

target_link_libraries(vpppack PRIVATE vpp_runtime)

if(MSVC)
  target_compile_options(vpppack PRIVATE /W4)
else()
  target_compile_options(vpppack PRIVATE -Wall -Wextra)
endif()
