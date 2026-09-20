ARG RUST_VERSION=1.90

FROM docker.io/library/rust:${RUST_VERSION}-alpine AS build

WORKDIR /ytchecker

RUN apk add --no-cache clang lld musl-dev git

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/var/cache/cargo \
    CARGO_HOME=/var/cache/cargo cargo build --release && \
    cp ./target/release/ytchecker /bin/ytchecker

FROM docker.io/library/alpine:3.18 AS final

RUN apk add --no-cache python3 py3-pip \
    && pip3 install --no-cache-dir -U "yt-dlp[default]"

ARG UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

USER appuser

COPY --from=build /bin/ytchecker /bin/ytchecker

ENTRYPOINT ["/bin/ytchecker"]