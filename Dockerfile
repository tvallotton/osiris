FROM rust:latest
WORKDIR /root/osiris
RUN mkdir /root/.config
COPY . .

