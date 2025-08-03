#!/bin/bash

# Based on https://gist.github.com/BaksiLi/ea2f505fdbe42349a5225390264c1f40

arch=$(uname -m)

filename="zellij-${arch}-unknown-linux-musl.tar.gz"
url="https://github.com/zellij-org/zellij/releases/latest/download/$filename"
echo "Downloading Zellij binary for Linux..."
curl -LO "$url"

# Uncompress the Zellij binary
echo "Uncompressing Zellij binary..."
tar -xf "$filename"

# Move the Zellij binary to the /bin directory
echo "Moving Zellij binary to /bin directory..."
mv "./zellij" /bin/zellij

# Remove the .tar.gz file
echo "Removing .tar.gz file..."
rm "$filename"

# Check if the Zellij binary exists
if [ -f "/bin/zellij" ]; then
  echo "Zellij binary installed successfully!"
else
  echo "Zellij binary not installed successfully!"
fi
