# Build 
FROM rust:alpine as builder
ARG TARGET_ARCH=x86_64-unknown-linux-musl

RUN apk add --no-cache musl-dev alpine-sdk
WORKDIR /usr/src/mavlink2rest
COPY . .

RUN rustup target add ${TARGET_ARCH}

RUN cargo build --release --target=${TARGET_ARCH}


# Runtime environment
FROM alpine
WORKDIR /root/

ARG TARGET_ARCH=x86_64-unknown-linux-musl

COPY --from=builder /usr/src/mavlink2rest/target/${TARGET_ARCH}/release/mavlink2rest ./mavlink2rest

ENV MAVLINK_SRC="udpin:0.0.0.0:14550"
ENV SERVER_PORT="0.0.0.0:8088"
ENV EXTRA_ARGS=""

RUN chmod +x mavlink2rest

ENTRYPOINT ./mavlink2rest -c ${MAVLINK_SRC} -s ${SERVER_PORT} ${EXTRA_ARGS}
