FROM rust:1.87-bookworm

WORKDIR /tmp

RUN ["useradd", "user"]
COPY Cargo.* ./
COPY src src
COPY benches benches
RUN ["chown", "user", "-R", "."]
USER user
RUN ["cargo", "update"]

CMD ["cargo", "build", "--release"]
