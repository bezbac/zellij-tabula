FROM ubuntu:latest

RUN apt update

# Install curl
RUN apt install curl -y

# Install Node.js and npm
RUN apt install -y nodejs npm

# Install zsh
RUN apt install zsh -y
RUN chsh -s $(which zsh)

# Install zellij
COPY ./tests/install_zellij.sh /install_zellij.sh
RUN chmod +x /install_zellij.sh
RUN /install_zellij.sh
RUN rm /install_zellij.sh

# Install zellij-tabula plugin
COPY ./zellij/target/wasm32-wasi/release/zellij-tabula.wasm /zellij-tabula.wasm
COPY ./zellij-tabula.plugin.zsh /zellij-tabula.plugin.zsh

# Configure zsh
RUN touch /root/.zshrc
RUN echo "PS1='%F%B%~%b%f %F$ %f'" >> /root/.zshrc
RUN echo 'source /zellij-tabula.plugin.zsh' >> /root/.zshrc
RUN echo 'echo "Using config /root/.zshrc\n"' >> /root/.zshrc

# Install npm dependencies for tests
WORKDIR /tests
COPY ./tests/package.json ./package.json
RUN npm install

# Configure zellij
COPY ./tests/config.kdl /root/.config/zellij/config.kdl

# Copy tests
COPY ./tests/example.test.js ./example.test.js

ENTRYPOINT ["./node_modules/.bin/tui-test"]
