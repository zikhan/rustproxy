FROM rust:1 as build
WORKDIR /src
COPY . .
RUN cargo build --release

FROM debian:buster-slim as final
WORKDIR /app
RUN apt-get update && apt-get install -y openssl ca-certificates
COPY --from=build /src/target/release/rustproxy .
EXPOSE 8080
ENTRYPOINT [ "./rustproxy" ]