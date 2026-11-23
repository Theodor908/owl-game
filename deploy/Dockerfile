# Builds the Owl server and packs it with the game's content.
# Build from the project root: docker compose -f deploy/docker-compose.yml build

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release -p owl-server

FROM debian:bookworm-slim
# The server runs as its own user. The database and the key file live in two
# separate places, so a copy of one never carries the other.
RUN useradd --system --uid 10001 --home /opt/owl owl \
 && mkdir -p /var/lib/owl /etc/owl \
 && chown owl: /var/lib/owl /etc/owl \
 && chmod 700 /etc/owl
WORKDIR /opt/owl
COPY --from=build /src/target/release/owl-server ./owl-server
COPY data data
COPY docs/privacy.md docs/privacy.md
USER owl
# Inside the container it listens on every address; the compose file
# publishes no port, so only the tunnel beside it can reach it.
ENV OWL_BIND=0.0.0.0:8080 \
    OWL_DB=/var/lib/owl/owl.db \
    OWL_KEY=/etc/owl/owl.key
VOLUME ["/var/lib/owl", "/etc/owl"]
EXPOSE 8080
CMD ["./owl-server"]
