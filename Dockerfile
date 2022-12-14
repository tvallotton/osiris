FROM rust:1.65
WORKDIR /root/osiris
RUN mkdir /root/.config
COPY . .
RUN cargo install cargo-watch
