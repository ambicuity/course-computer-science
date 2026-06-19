# Lesson Dockerfile — a small but useful CS toolchain image.
# Build:  docker build -t cs-toolchain-demo .
# Run:    docker run --rm -it cs-toolchain-demo

# 1. Pin a specific base image — never use :latest in production.
FROM debian:bookworm-slim

# 2. Install system packages in ONE layer, clean the apt cache to avoid bloat.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        gdb \
        valgrind \
        ca-certificates \
        curl \
        git \
        pkg-config && \
    rm -rf /var/lib/apt/lists/*

# 3. Install Rust as the *student* user, not root.
RUN useradd --create-home --shell /bin/bash student
USER student
ENV PATH=/home/student/.cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable --profile minimal && \
    rustc --version

# 4. Workdir for course material. Don't COPY the entire host into the image;
#    devcontainer.json will bind-mount the repo at runtime.
WORKDIR /home/student/course

# 5. Default command.
CMD ["bash"]
