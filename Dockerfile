FROM debian
RUN apt-get update
RUN apt-get install linux-perf -y
RUN apt-get install git -y
RUN apt-get install curl -y
RUN apt-get install build-essential -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN /root/.cargo/bin/cargo install flamegraph
RUN /root/.cargo/bin/cargo install cargo-llvm-cov
WORKDIR /root/osiris
# RUN mkdir /root/.config
# RUN echo 0 > /proc/sys/kernel/kptr_restrict
# RUN apk add git
COPY . .
