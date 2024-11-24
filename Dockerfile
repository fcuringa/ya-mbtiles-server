FROM rust:1.82.0-bookworm as build
LABEL authors="Florian Curinga"

WORKDIR /src

RUN apt update
RUN apt install -y python3 python3-dev

COPY Cargo.* .
COPY src src

RUN cargo build --profile release

COPY example_auth.py auth.py

EXPOSE 3000

ENTRYPOINT ["/src/target/release/ya-mbtiles-server"]
