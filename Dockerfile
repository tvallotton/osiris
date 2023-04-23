FROM rust:1.66
WORKDIR /root/osiris
RUN mkdir /root/.config
COPY . .
RUN cargo install cargo-watch
