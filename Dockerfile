# ETAPA 1: Constructor (Builder)
# Usamos una imagen oficial de Rust para compilar
FROM rust:1.75-slim-bookworm as builder

# Instalamos dependencias del sistema necesarias para compilar (OpenSSL)
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Creamos un proyecto vacío para cachear las dependencias
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Creamos un main dummy para que compile solo las librerías primero
mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Ahora copiamos TU código real
COPY . .
# Forzamos la actualización del archivo main para que lo recompile
RUN touch src/main.rs
RUN cargo build --release

# ETAPA 2: Ejecución (Runtime)
# Usamos una imagen ligera de Debian para correr el programa
FROM debian:bookworm-slim

# Instalamos certificados SSL (necesarios para conectarse a Mongo Atlas)
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiamos el binario desde la etapa anterior
COPY --from=builder /app/target/release/imanes_nfc .

# ¡IMPORTANTE! Copiamos las carpetas de templates y static
COPY templates/ ./templates/
# COPY static/ ./static/  <-- Descomenta si usas la carpeta static

# Exponemos el puerto
EXPOSE 3000

# Comando para iniciar
CMD ["./imanes_nfc"]
