// main.cpp — RAII in C++: FileHandle, unique_ptr, exception safety.
// Build: g++ -std=c++17 -O0 -g main.cpp -o main && ./main

#include <cstdio>
#include <cstdlib>
#include <memory>
#include <stdexcept>
#include <iostream>

class FileHandle {
    FILE *fp_;
public:
    FileHandle(const char *path, const char *mode) {
        fp_ = std::fopen(path, mode);
        if (!fp_) throw std::runtime_error("fopen failed");
        std::cout << "  FileHandle(" << path << ") opened\n";
    }
    ~FileHandle() {
        if (fp_) {
            std::fclose(fp_);
            std::cout << "  FileHandle destructor: closed\n";
        }
    }
    FileHandle(const FileHandle&) = delete;             // non-copyable
    FileHandle& operator=(const FileHandle&) = delete;
    FileHandle(FileHandle&& o) noexcept : fp_(o.fp_) { o.fp_ = nullptr; }

    FILE *get() const { return fp_; }
};

struct Widget {
    int id;
    Widget(int i) : id(i) { std::cout << "  Widget(" << id << ") created\n"; }
    ~Widget()              { std::cout << "  Widget(" << id << ") destroyed\n"; }
};

void exception_safe_demo() {
    std::cout << "\n  Entering exception_safe_demo\n";
    FileHandle f("/tmp/lesson_raii.txt", "w");
    // Simulate an error after acquiring f:
    throw std::runtime_error("simulated failure");
}

int main() {
    std::cout << "== RAII: FileHandle ==\n";
    {
        const char *tmp = "/tmp/lesson_raii.txt";
        FILE *w = std::fopen(tmp, "w"); if (w) { std::fprintf(w, "demo\n"); std::fclose(w); }
        FileHandle fh(tmp, "r");
        char buf[16];
        if (std::fgets(buf, sizeof(buf), fh.get())) {
            std::cout << "  read: " << buf;
        }
        // ~FileHandle runs at the end of this scope
    }

    std::cout << "\n== Exception safety (destructor still runs) ==\n";
    try {
        exception_safe_demo();
    } catch (const std::exception &e) {
        std::cout << "  caught: " << e.what() << "\n";
    }

    std::cout << "\n== Smart pointer: unique_ptr ==\n";
    {
        auto p = std::make_unique<Widget>(42);
        // No delete needed.
    } // Widget destructed here

    std::cout << "\n== Drop order: reverse declaration ==\n";
    {
        Widget a(1);
        Widget b(2);
        Widget c(3);
        // Destroys c, then b, then a — reverse order of declaration
    }

    std::cout << "\nReturn from main\n";
    std::remove("/tmp/lesson_raii.txt");
    return 0;
}
