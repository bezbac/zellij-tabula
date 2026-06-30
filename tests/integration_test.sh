# Set the current working directory to the root of the repository,
# regardless of where the script is run from
cd "$(git rev-parse --show-toplevel)"

# Build latest version of the plugin
cd ./zellij
cargo build --release

# Build the docker image
cd ..

# Pre-flight: ensure all mounted host files exist
wasm=./zellij/target/wasm32-wasip1/release/zellij-tabula.wasm
plugin=./zellij-tabula.plugin.zsh
config=./tests/config.kdl
for f in "$wasm" "$plugin" "$config" ./tests/package.json; do
  [ -f "$f" ] || { echo "Missing file: $f" >&2; exit 1; }
done

docker build -t zellij:test -f ./tests/Dockerfile .

# Run the expect script inside the docker container.
# ./tests is bind-mounted; an anonymous volume shields /tests/node_modules
# (baked into the image) so the bind mount doesn't shadow it.
docker run --rm -t \
  -v "$wasm":/zellij-tabula.wasm:ro \
  -v "$plugin":/zellij-tabula.plugin.zsh:ro \
  -v "$config":/home/alice/.config/zellij/config.kdl:ro \
  -v ./tests:/tests \
  -v /tests/node_modules \
  zellij:test

# Delete the docker image after the test is done
docker rmi zellij:test
