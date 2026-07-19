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

# Configure zsh
RUN touch /home/alice/.zshrc
RUN echo "PS1='%F%B%~%b%f %F$ %f'" >> /home/alice/.zshrc
RUN echo 'echo "Using config /home/alice/.zshrc\n"' >> /home/alice/.zshrc
RUN chown alice:alice /home/alice/.zshrc

# Install npm dependencies for tests
WORKDIR /tests
COPY ./tests/package.json ./package.json
RUN npm install

# Configure zellij (mount target dirs must exist)
RUN mkdir -p /home/alice/.config/zellij

USER alice

ENTRYPOINT ["./node_modules/.bin/tui-test"]
