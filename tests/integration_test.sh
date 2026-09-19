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
config=./tests/config.kdl
for f in "$wasm" "$config" ./tests/package.json; do
  [ -f "$f" ] || { echo "Missing file: $f" >&2; exit 1; }
done

docker build -t zellij:test -f ./tests/Dockerfile . 2>&1 | tail -1


docker run --rm -t \
  -v "$wasm":/zellij-tabula.wasm:ro \
  -v "$config":/home/alice/.config/zellij/config.kdl:ro \
  -v ./tests:/tests \
  -v /tests/node_modules \
  --entrypoint ./node_modules/.bin/tui-test \
  zellij:test \
  tests/main.test.js tests/pane-status.test.js tests/close-pane.test.js tests/stable-id.test.js

status=$?

docker rmi zellij:test 2>/dev/null || true

exit $status
