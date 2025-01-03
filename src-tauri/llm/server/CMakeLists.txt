# Minimum CMake version required
cmake_minimum_required(VERSION 3.10)

# Project name and version
project(Qwen2VLServer VERSION 1.0 LANGUAGES CXX)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Add the llama.cpp library
add_subdirectory(${CMAKE_SOURCE_DIR}/../llama.cpp llama_lib)

# Collect source files using wildcards
file(GLOB COMMON_SOURCES "${CMAKE_SOURCE_DIR}/../llama.cpp/common/*.cpp")

# Add the executable with sources
set(TARGET qwen2vl-server)
add_executable(${TARGET}
    ${CMAKE_SOURCE_DIR}/src/server.cpp
    ${CMAKE_SOURCE_DIR}/../llama.cpp/examples/llava/clip.cpp
    ${CMAKE_SOURCE_DIR}/../llama.cpp/examples/llava/llava.cpp
    ${COMMON_SOURCES}
)
set_target_properties(${TARGET} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
)
install(TARGETS ${TARGET} RUNTIME)
target_link_libraries(${TARGET} PRIVATE 
    llama 
    ggml
)
# Specify include directories for the executable
target_include_directories(${TARGET} PRIVATE
    ${CMAKE_SOURCE_DIR}/../llama.cpp
    ${CMAKE_SOURCE_DIR}/../llama.cpp/common
    ${CMAKE_SOURCE_DIR}/../llama.cpp/examples/llava
    ${CMAKE_SOURCE_DIR}/../llama.cpp/examples/server
    ${CMAKE_SOURCE_DIR}/include
)
target_compile_features(${TARGET} PRIVATE cxx_std_17)
