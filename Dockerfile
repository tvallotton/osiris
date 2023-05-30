FROM rust
WORKDIR /root/osiris
RUN mkdir /root/.config
RUN rustup component add clippy
RUN rustup component add rustfmt
RUN cargo install cargo-llvm-cov
RUN cargo install flamegraph
COPY . .
