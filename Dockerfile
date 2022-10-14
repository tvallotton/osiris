FROM rust:1.64
WORKDIR /home/osiris
COPY . .
RUN cargo install cargo-watch
