cmake_minimum_required(VERSION 3.15)
project(hello LANGUAGES C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Debug)
endif()

add_library(greet STATIC greet.c)
target_include_directories(greet PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

add_executable(hello hello.c)
target_link_libraries(hello PRIVATE greet)

enable_testing()
add_test(NAME hello_runs COMMAND hello)
set_tests_properties(hello_runs PROPERTIES
    PASS_REGULAR_EXPRESSION "hello, world"
)
