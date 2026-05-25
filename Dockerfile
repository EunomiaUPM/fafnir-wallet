# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.88.0
ARG APP_NAME=fafnir-wallet

# --- backend-builder ---
FROM rust:${RUST_VERSION}-slim-bookworm AS backend-builder
ARG APP_NAME
WORKDIR /app
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
RUN --mount=type=bind,source=Cargo.toml,target=/app/Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=/app/Cargo.lock \
    --mount=type=bind,source=src,target=/app/src \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    /bin/sh -c "\
    set -e; \
    cargo build --locked --release -p ${APP_NAME}; \
    cp ./target/release/${APP_NAME} /bin/${APP_NAME}; \
"

# --- final ---
FROM debian:bookworm-slim AS final
ARG APP_NAME
ENV APP_NAME=${APP_NAME}
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

WORKDIR /app
COPY --from=backend-builder /bin/${APP_NAME} /usr/local/bin/
RUN chmod +x /usr/local/bin/${APP_NAME}

# Config por defecto pensada para correr siempre como contenedor
# (url: 'fafnir-wallet' = nombre de servicio docker-compose).
# Para usar otro yaml: o montas un volumen sobre esta misma ruta,
# o sobrescribes `command:` en el compose.
COPY static/environment/config/prod/fafnir.yaml /app/static/environment/config/prod/fafnir.yaml

# Carpeta de persistencia + propiedad de /app para `appuser` no-root.
# Sin esto, `create_dir_all("/app/data")` revienta con Permission
# denied (porque `WORKDIR /app` se crea por defecto como root).
RUN mkdir -p /app/data && chown -R appuser:appuser /app

USER appuser
EXPOSE 7001
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/fafnir-wallet"]
CMD ["start", "--env-file", "static/environment/config/prod/fafnir.yaml"]
