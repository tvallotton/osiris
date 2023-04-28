FROM rust:latest
WORKDIR /root/osiris
RUN mkdir /root/.config
RUN rustup component add clippy
RUN rustup component add rustfmt
COPY . .

