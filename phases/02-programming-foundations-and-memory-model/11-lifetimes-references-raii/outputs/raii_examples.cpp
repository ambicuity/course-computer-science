// raii_examples.cpp — three RAII patterns ready to drop into any C++ project.
//
// Build:  g++ -std=c++17 -O0 -g raii_examples.cpp -o raii && ./raii

#include <cstdio>
#include <functional>
#include <iostream>
#include <mutex>
#include <stdexcept>

// ── 1. FileHandle ─────────────────────────────────────────────────

class FileHandle {
    FILE *fp_;
public:
    FileHandle(const char *path, const char *mode) {
        fp_ = std::fopen(path, mode);
        if (!fp_) throw std::runtime_error("fopen failed");
    }
    ~FileHandle() { if (fp_) std::fclose(fp_); }

    FileHandle(const FileHandle&) = delete;
    FileHandle& operator=(const FileHandle&) = delete;
    FileHandle(FileHandle&& o) noexcept : fp_(o.fp_) { o.fp_ = nullptr; }

    FILE *get() const { return fp_; }
};

// ── 2. MutexGuard — scope-bound lock ─────────────────────────────

template <typename Mutex>
class MutexGuard {
    Mutex &m_;
public:
    explicit MutexGuard(Mutex &m) : m_(m) { m_.lock();   }
    ~MutexGuard()                          { m_.unlock(); }
    MutexGuard(const MutexGuard&) = delete;
};

// ── 3. ScopeGuard — run a lambda at scope exit ───────────────────

class ScopeGuard {
    std::function<void()> fn_;
    bool active_ = true;
public:
    explicit ScopeGuard(std::function<void()> fn) : fn_(std::move(fn)) {}
    ~ScopeGuard() { if (active_) fn_(); }
    void dismiss() { active_ = false; }
    ScopeGuard(const ScopeGuard&) = delete;
};

#define ON_SCOPE_EXIT(stmt) ScopeGuard _scope_guard_##__LINE__([&]() { stmt; })

// ── Demo ─────────────────────────────────────────────────────────

int main() {
    std::cout << "== FileHandle ==\n";
    {
        const char *tmp = "/tmp/raii_examples.txt";
        FILE *w = std::fopen(tmp, "w"); if (w) { std::fprintf(w, "hi\n"); std::fclose(w); }
        FileHandle f(tmp, "r");
        char buf[8];
        if (std::fgets(buf, sizeof(buf), f.get())) std::cout << "  read: " << buf;
        std::remove(tmp);
    }
    std::cout << "  (file auto-closed)\n";

    std::cout << "\n== MutexGuard ==\n";
    std::mutex m;
    {
        MutexGuard<std::mutex> guard(m);
        std::cout << "  inside critical section (lock held)\n";
    }
    std::cout << "  lock released by guard's destructor\n";

    std::cout << "\n== ScopeGuard ==\n";
    {
        int *buf = (int*)std::malloc(100 * sizeof(int));
        ON_SCOPE_EXIT(std::free(buf); std::cout << "  ↓ ScopeGuard freed buf\n");
        std::cout << "  using buf at " << (void*)buf << "\n";
        // Even if we threw here, ScopeGuard's dtor would free.
    }

    return 0;
}
