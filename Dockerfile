FROM rust:1.65
WORKDIR /home/osiris
COPY . .
RUN cargo install cargo-watch
