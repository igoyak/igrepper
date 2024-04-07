FROM rust:1.77.1-buster

WORKDIR /tmp

RUN ["useradd", "user"]
COPY Cargo.* ./
COPY src src
RUN ["chown", "user", "-R", "."]
USER user
RUN ["cargo", "update"]

CMD ["cargo", "build", "--release"]
