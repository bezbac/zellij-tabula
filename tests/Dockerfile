# syntax=docker/dockerfile:1

# Build the Rust integration-test binary (a standalone runner) on a pinned
# toolchain, then copy it into the runtime image that has zsh + zellij.
FROM rust:1.96.1 AS builder
WORKDIR /app

COPY ./tests/Cargo.toml ./tests/Cargo.lock ./
COPY ./tests/src ./src
RUN cargo build --release

FROM ubuntu:latest

RUN apt update

# Install curl, git, zsh
RUN apt install -y curl git zsh

RUN useradd --create-home --shell /usr/bin/zsh alice
ENV HOME=/home/alice
ENV USER=alice

# Install zellij
COPY ./tests/install_zellij.sh /install_zellij.sh
RUN chmod +x /install_zellij.sh
RUN /install_zellij.sh
RUN rm /install_zellij.sh

# Configure zsh
RUN touch /home/alice/.zshrc
RUN echo "PS1='%F%B%~%b%f %F$ %f'" >> /home/alice/.zshrc
RUN echo 'echo "Using config /home/alice/.zshrc\n"' >> /home/alice/.zshrc
RUN chown alice:alice /home/alice/.zshrc

# Configure zellij (mount target dirs must exist)
RUN mkdir -p /home/alice/.config/zellij

# Pre-populate the permission cache so zellij loads zellij-tabula without a live prompt
COPY ./tests/permissions.kdl /home/alice/.cache/zellij/permissions.kdl
RUN chown alice:alice /home/alice/.cache/zellij/permissions.kdl

COPY --from=builder /app/target/release/integration-tests /usr/local/bin/integration-tests

USER alice

ENTRYPOINT ["/usr/local/bin/integration-tests"]