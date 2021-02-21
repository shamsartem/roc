FROM rust:1.49-slim-buster
WORKDIR /earthbuild

prep-debian:
    RUN apt -y update

install-other-libs:
    FROM +prep-debian
    RUN apt -y install wget git
    RUN apt -y install libxcb-shape0-dev libxcb-xfixes0-dev # for editor clipboard
    RUN apt -y install libc++-dev libc++abi-dev libunwind-dev pkg-config libx11-dev zlib1g-dev

install-zig-llvm-valgrind-clippy-rustfmt:
    FROM +install-other-libs
    # zig
    RUN wget -c https://ziglang.org/download/0.7.1/zig-linux-x86_64-0.7.1.tar.xz --no-check-certificate
    RUN tar -xf zig-linux-x86_64-0.7.1.tar.xz
    RUN ln -s /earthbuild/zig-linux-x86_64-0.7.1/zig /usr/bin/zig
    # llvm
    RUN apt -y install lsb-release software-properties-common gnupg
    RUN wget https://apt.llvm.org/llvm.sh
    RUN chmod +x llvm.sh
    RUN ./llvm.sh 10
    RUN ln -s /usr/bin/clang-10 /usr/bin/clang
    # use lld as linker
    RUN ln -s /usr/bin/lld-10 /usr/bin/ld.lld
    RUN echo "[build]" > $CARGO_HOME/config.toml
    RUN echo "rustflags = [\"-C\", \"link-arg=-fuse-ld=lld\", \"-C\", \"target-cpu=native\"]" >> $CARGO_HOME/config.tom
    # valgrind
    RUN apt -y install autotools-dev cmake automake libc6-dbg
    RUN wget https://sourceware.org/pub/valgrind/valgrind-3.16.1.tar.bz2
    RUN tar -xf valgrind-3.16.1.tar.bz2
    # need to cd every time, every command starts at WORKDIR
    RUN cd valgrind-3.16.1; ./autogen.sh
    RUN cd valgrind-3.16.1; ./configure --disable-dependency-tracking
    RUN cd valgrind-3.16.1; make -j`nproc`
    RUN cd valgrind-3.16.1; make install
    # clippy
    RUN rustup component add clippy
    # rustfmt
    RUN rustup component add rustfmt
    # sccache
    RUN apt install libssl-dev
    RUN cargo install sccache
    RUN sccache -V

deps-image:
    FROM +install-zig-llvm-valgrind-clippy-rustfmt
    SAVE IMAGE roc-deps:latest

copy-dirs-and-cache:
    FROM +install-zig-llvm-valgrind-clippy-rustfmt
    # cache
    COPY --dir sccache_dir ./
    # roc dirs
    COPY --dir cli compiler docs editor roc_std vendor examples Cargo.toml Cargo.lock ./

test-zig:
    FROM +install-zig-llvm-valgrind-clippy-rustfmt
    COPY --dir compiler/builtins/bitcode ./
    RUN cd bitcode; ./run-tests.sh;

build-rust:
    FROM +copy-dirs-and-cache
    ARG RUSTC_WRAPPER=/usr/local/cargo/bin/sccache
    ARG SCCACHE_DIR=/earthbuild/sccache_dir
    ARG CARGO_INCREMENTAL=0 # no need to recompile package when using new function
    RUN cargo build; sccache --show-stats # for clippy
    RUN cargo test --release --no-run; sccache --show-stats

check-clippy:
    FROM +build-rust
    RUN cargo clippy -V
    RUN cargo clippy -- -D warnings

check-rustfmt:
    FROM +copy-dirs-and-cache
    RUN cargo fmt --version
    RUN cargo fmt --all -- --check

save-cache:
    FROM +build-rust
    SAVE ARTIFACT sccache_dir AS LOCAL sccache_dir

test-rust:
    FROM +build-rust
    ARG RUSTC_WRAPPER=/usr/local/cargo/bin/sccache
    ARG SCCACHE_DIR=/earthbuild/sccache_dir
    RUN cargo test --release 

test-all:
    BUILD +check-clippy
    BUILD +check-rustfmt
    BUILD +save-cache
    BUILD +test-zig
    BUILD +test-rust
    