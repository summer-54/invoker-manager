FROM rust:latest

WORKDIR /app

COPY ./Cargo.toml ./
COPY ./src ./src

ENV RUST_LOG=invoker_manager=trace
ENV INVOKERS_ADDRESS=0.0.0.0:1111
ENV TS_ADDRESS=0.0.0.0:2222
ENV CP_ADDRESS=0.0.0.0:3333
ENV RUST_BACKTRACE=1

RUN ["apt-get", "update"]
RUN ["apt-get", "install", "clang", "--no-install-recommends", "-y"]

RUN ["cargo", "build", "--release"]

CMD ["cargo", "run", "--release"]
