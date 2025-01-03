# How to build the Qwen server

1. Make and navigate to the build directory<br />
    ```mkdir build && cd build```

2. Set up CMake for the project<br />
    ```cmake ..```

3. Build the project<br />
    ```cmake --build . --config Release```

Once you set up cmake, you only have to rebuild the project for edits made to server.cpp.