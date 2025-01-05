#include <iostream>
#include <string>
#include <thread>
#include <atomic>
#include <fstream>

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
                std::cout << response << std::endl;
            } else {
                std::cout << "ERROR - unknown function: " << input << std::endl; 
            }
            input = "";
        }
    }
}

int main() {
    std::atomic<bool> running(true);
    std::thread listener(processRequest, std::ref(running));

    listener.join();

    return 0;
}