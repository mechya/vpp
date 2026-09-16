# vpp_updater: fetching applications over HTTP(S), the local application
# store, and incremental updates by resource hash.
add_library(vpp_updater STATIC
  src/http.cpp
  src/updater.cpp
)

target_include_directories(vpp_updater PUBLIC include)
target_link_libraries(vpp_updater PUBLIC vpp_runtime)

if(WIN32)
  target_link_libraries(vpp_updater PRIVATE winhttp)
endif()

if(MSVC)
  target_compile_options(vpp_updater PRIVATE /W4)
else()
  target_compile_options(vpp_updater PRIVATE -Wall -Wextra)
endif()
