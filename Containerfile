FROM rust:latest
COPY . .
ENV RUST_LOG=trace
ENV INVOKERS_ADDRESS=123
ENV TS_ADDRESS=123
RUN ["cargo", "build", "--release"]
CMD ["cargo", "run", "--release"]
