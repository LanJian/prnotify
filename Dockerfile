FROM debian:buster-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends libsqlite3-dev
COPY ./target/x86_64-unknown-linux-musl/release/prnotify ./prnotify
CMD ["/prnotify"]
