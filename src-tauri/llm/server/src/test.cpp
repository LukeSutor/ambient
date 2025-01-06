#include <iostream>
#include <string>
#include <thread>
#include <atomic>
#include <fstream>

// Helper function to restore stdout and stderr
void restore_stdout_stderr() {
    #ifdef _WIN32
        freopen("CON", "w", stdout);
        freopen("CON", "w", stderr);
    #else
        freopen("/dev/tty", "w", stdout);
        freopen("/dev/tty", "w", stderr);
    #endif
}

// Helper function to redirect stdout and stderr to null
void redirect_stdout_stderr_to_null() {
#ifdef _WIN32
    freopen("NUL", "w", stdout);
    freopen("NUL", "w", stderr);
#else
    freopen("/dev/null", "w", stdout);
    freopen("/dev/null", "w", stderr);
#endif
}

std::string infer(const std::string& data) {
    return "Inferred answer based on: " + data;
}

std::string load_model(const std::string& data) {
    return "Model loaded with data: " + data;
}

void log_input(const std::string& input) {
    std::ofstream log_file("C:\\Users\\Luke\\Downloads\\log.txt", std::ios_base::app);
    if (log_file.is_open()) {
        log_file << input << std::endl;
    }
}

void processRequest(std::atomic<bool>& running) {
    std::string input;
    while (running) {
        std::getline(std::cin, input);
        std::cin.clear(); // Clear the input buffer
        if (!input.empty()) {
            log_input(input); // Log the input
            if (input == "SHUTDOWN") {
                std::cout << "Shutting down..." << std::endl;
                running = false;
                break;
            } else if (input.rfind("INFER", 0) == 0) {
                std::string response = infer(input.substr(6)); // Call infer with the rest of the input
                std::cout << response << std::endl;
            } else if (input.rfind("LOAD", 0) == 0) {
                std::string response = load_model(input.substr(5)); // Call load_model with the rest of the input
                log_input("BEFORE PRINT");
                std::cout << response << std::endl;
                log_input("AFTER PRINT");
            } else {
                std::cout << "ERROR - unknown function: " << input << std::endl; 
            }
            input = "";
        }
    }
}

int main() {
    // redirect_stdout_stderr_to_null();
    // restore_stdout_stderr();
    std::atomic<bool> running(true);
    std::thread listener(processRequest, std::ref(running));

    listener.join();

    return 0;
}