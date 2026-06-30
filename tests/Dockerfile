FROM ubuntu:latest

RUN apt update

# Install curl
RUN apt install curl -y

# Install git
RUN apt install git -y

# Install Node.js and npm
RUN apt install -y nodejs npm

# Install zsh
RUN apt install zsh -y
RUN useradd --create-home --shell $(which zsh) alice
ENV HOME=/home/alice
ENV USER=alice

# Install zellij
COPY ./tests/install_zellij.sh /install_zellij.sh
RUN chmod +x /install_zellij.sh
RUN /install_zellij.sh
RUN rm /install_zellij.sh

# Install zellij-tabula plugin
COPY ./zellij/target/wasm32-wasip1/release/zellij-tabula.wasm /zellij-tabula.wasm
COPY ./zellij-tabula.plugin.zsh /zellij-tabula.plugin.zsh

# Configure zsh
RUN touch /home/alice/.zshrc
RUN echo "PS1='%F%B%~%b%f %F$ %f'" >> /home/alice/.zshrc
RUN echo 'source /zellij-tabula.plugin.zsh' >> /home/alice/.zshrc
RUN echo 'echo "Using config /home/alice/.zshrc\n"' >> /home/alice/.zshrc
RUN chown alice:alice /home/alice/.zshrc

# Install npm dependencies for tests
WORKDIR /tests
COPY ./tests/package.json ./package.json
COPY ./tests/tui-test.config.js ./tui-test.config.js
RUN npm install

# Configure zellij
RUN mkdir -p /home/alice/.config/zellij
COPY ./tests/config.kdl /home/alice/.config/zellij/config.kdl
RUN chown -R alice:alice /home/alice/.config

# Copy tests
COPY ./tests/package.json ./package.json
COPY ./tests/tui-test.config.js ./tui-test.config.js
COPY ./tests/main.test.js ./main.test.js
COPY ./tests/session-rename.test.js ./session-rename.test.js
COPY ./tests/stable-id.test.js ./stable-id.test.js
COPY ./tests/test-utils.js ./test-utils.js
RUN chown -R alice:alice /tests

USER alice

ENTRYPOINT ["./node_modules/.bin/tui-test"]
