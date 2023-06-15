FROM debian:buster-slim
COPY ./target/x86_64-unknown-linux-musl/release/prnotify ./prnotify
CMD ["/prnotify"]
