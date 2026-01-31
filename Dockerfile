# ETAPA 1: Constructor (Builder)
# Usamos la versión MÁS RECIENTE de Rust para evitar errores de compatibilidad
FROM rust:latest as builder

# Instalamos dependencias del sistema (OpenSSL es vital para Mongo)
RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /app

# Truco de caché: Creamos un proyecto vacío para compilar solo las librerías primero
RUN cargo new --bin imanes_nfc
WORKDIR /app/imanes_nfc

# Copiamos tus archivos de configuración
COPY Cargo.toml Cargo.lock ./

# Compilamos SOLO las dependencias (esto tardará la primera vez, luego vuela)
RUN cargo build --release

# Ahora borramos el código basura y copiamos TU código real
RUN rm src/*.rs
COPY ./src ./src
COPY ./templates ./templates
# COPY ./static ./static  <-- Descomenta si usas static

# Borramos el ejecutable anterior para forzar la recompilación con tu código
RUN rm ./target/release/deps/imanes_nfc*
RUN cargo build --release

# ETAPA 2: Ejecución (Runtime) Final
FROM debian:bookworm-slim

# Instalamos certificados SSL para que pueda hablar con Mongo Atlas
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiamos el binario compilado (Asegúrate que en Cargo.toml tu name="imanes_nfc")
COPY --from=builder /app/imanes_nfc/target/release/imanes_nfc .
COPY --from=builder /app/imanes_nfc/templates ./templates

# Exponemos el puerto
ENV SERVER_PORT=3000
EXPOSE 3000

# Arrancamos la nave
CMD ["./imanes_nfc"]
